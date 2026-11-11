use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair, KeyUsagePurpose};
use std::{fs, process::Command};

#[test]
fn tls_round_trip_and_certificate_validation() {
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    let ca_key = KeyPair::generate().unwrap();
    let ca = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::new(ca_params, ca_key);
    let server_key = KeyPair::generate().unwrap();
    let server = CertificateParams::new(vec!["localhost".into()])
        .unwrap()
        .signed_by(&server_key, &issuer)
        .unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(project.path().join("i.toml"), "name = \"tls_test\"\n").unwrap();
    let source = format!(
        r#"
(
 i/net
 i/tls
 i/task
 i/fail
)
@main
main = () {{
 listener = net.listen("127.0.0.1", 0)
 port = net.local_port(listener)
 ident = tls.identity(<<{server:?}:utf8>>, <<{key:?}:utf8>>)
 opts = (..tls.options(), alpn: ["test"], roots: <<{ca:?}:utf8>>)
 task.run(() {{
  task.scope((scope) {{
   task.go(scope, () {{
    stream = tls.accept(net.accept(listener), ident, opts)
    assert(tls.alpn(stream) == "test")
    assert(tls.read(stream, 4) == <<"ping":utf8>>)
    tls.write(stream, <<"pong":utf8>>)
    tls.close(stream)
   }})
   stream = tls.dial("localhost", port, opts)
   assert(tls.alpn(stream) == "test")
   flow = tls.byte_stream(stream)
   flow.write(<<"ping":utf8>>)
   assert(flow.read(4) == <<"pong":utf8>>)
   tls.close(stream)
   Nil
  }})
 }})
 task.run(() {{
  task.scope((scope) {{
   server = task.go(scope, () {{ fail.catch(() {{ tls.accept(net.accept(listener), ident, opts) }}) }})
   result = fail.catch(() {{ tls.dial("localhost", port, tls.options()) }})
   result |> {{ Error(tls.TlsError(stage, reason)) -> assert(stage == "certificate") _ -> panic("expected certificate error") }}
   task.join(server)
   Nil
  }})
 }})
 task.run(() {{
  task.scope((scope) {{
   server = task.go(scope, () {{ fail.catch(() {{ tls.accept(net.accept(listener), ident, opts) }}) }})
   result = fail.catch(() {{ tls.connect(net.connect("localhost", port), "wrong.example", opts) }})
   result |> {{ Error(tls.TlsError(stage, reason)) -> assert(stage == "certificate") _ -> panic("expected name error") }}
   task.join(server)
   Nil
  }})
 }})
 net.close_listener(listener)
}}
"#,
        server = server.pem(),
        key = server_key.serialize_pem(),
        ca = ca.pem()
    );
    fs::write(project.path().join("src/main.i"), source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--project"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
