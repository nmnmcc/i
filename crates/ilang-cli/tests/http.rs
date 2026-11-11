use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn request(port: u16, bytes: &[u8]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    stream.write_all(bytes).unwrap();
    let mut reply = String::new();
    stream.read_to_string(&mut reply).unwrap();
    reply
}

#[test]
fn http_server() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/http-server");
    let child = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--project"])
        .arg(project)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut server = Server(child);
    let mut line = String::new();
    BufReader::new(server.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let port: u16 = line
        .strip_prefix("listening ")
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let health = b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    assert!(request(port, health).ends_with("\r\n\r\nok"));
    let body = r#"{"name":"Ada","age":35}"#;
    let post = format!("POST /users HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len());
    assert!(request(port, post.as_bytes()).ends_with(r#"{"id":1,"name":"Ada"}"#));
    let body = r#"{"name":"Ada","age":"bad"}"#;
    let post = format!("POST /users HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len());
    let invalid = request(port, post.as_bytes());
    assert!(invalid.starts_with("HTTP/1.1 422"));
    assert!(invalid.contains("age"));
    let pipeline = b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\nGET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    assert_eq!(request(port, pipeline).matches("HTTP/1.1 200").count(), 2);
    let chunked = b"POST /users HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n17\r\n{\"name\":\"Ada\",\"age\":35}\r\n0\r\n\r\n";
    assert!(request(port, chunked).ends_with(r#"{"id":1,"name":"Ada"}"#));
    assert!(request(port, b"BAD\r\n\r\n").starts_with("HTTP/1.1 400"));
    let mut slow = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let worker = thread::spawn(move || {
        for byte in health {
            slow.write_all(&[*byte]).unwrap();
            thread::sleep(Duration::from_millis(50));
        }
    });
    let workers: Vec<_> = (0..20)
        .map(|_| thread::spawn(move || request(port, health)))
        .collect();
    for worker in workers {
        assert!(worker.join().unwrap().ends_with("\r\n\r\nok"));
    }
    assert!(
        !worker.is_finished(),
        "slow head completed before concurrent requests"
    );
    worker.join().unwrap();
}
