//! `i lsp` over standard input and output.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn send(w: &mut impl Write, body: &str) {
    write!(w, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    w.flush().unwrap();
}

/// The body of the next message.
fn receive(r: &mut impl BufRead) -> String {
    let mut length = 0;
    loop {
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(n) = line.strip_prefix("Content-Length: ") {
            length = n.parse().unwrap();
        }
    }
    let mut body = vec![0; length];
    r.read_exact(&mut body).unwrap();
    String::from_utf8(body).unwrap()
}

#[test]
fn initialize_and_exit() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_i"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );
    let init = receive(&mut stdout);
    assert!(init.contains(r#""id":1"#), "{init}");
    assert!(init.contains(r#""hoverProvider":true"#), "{init}");
    assert!(init.contains(r#""name":"i""#), "{init}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#,
    );
    let shutdown = receive(&mut stdout);
    assert!(shutdown.contains(r#""id":2"#), "{shutdown}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"exit","params":null}"#,
    );
    drop(stdin);
    assert!(child.wait().unwrap().success());
}
