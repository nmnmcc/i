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
    let post = format!(
        "POST /users HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    assert!(request(port, post.as_bytes()).ends_with(r#"{"id":1,"name":"Ada"}"#));
    let body = r#"{"name":"Ada","age":"bad"}"#;
    let post = format!(
        "POST /users HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
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

fn run_http_program(source: &str) {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    let std_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    std::fs::write(
        project.path().join("i.toml"),
        format!(
            "name = \"http_test\"\n[dependencies]\nstd = {{ path = {:?} }}\n",
            std_path.to_str().unwrap()
        ),
    )
    .unwrap();
    std::fs::write(project.path().join("src/main.i"), source).unwrap();
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

#[test]
fn http_timeout() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(2));
    });
    run_http_program(&format!(
        r#"
(
 i/fail
 i/task
 std/http
)
main = () {{
 task.run(() {{
  result = fail.catch(() {{ http.send_timeout(http.client().send, http.request(http.Method("GET"), "http://127.0.0.1:{port}/"), 100000000) }})
  result |> (Error(http.Timeout)) {{ Nil }} | (_) {{ panic("expected timeout") }}
 }})
}}
"#
    ));
    server.join().unwrap();
}

#[test]
fn http_dns() {
    run_http_program(
        r#"(
	i/fail
	std/http
)

main = () {
	result = fail.catch(() { http.get("http://name.invalid/") })
	result |> (Error(http.Dns(_))) { Nil } | (_) { panic("expected DNS error") }
}
"#,
    );
}

#[test]
fn http_dns_distinct_from_connection_refused() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    run_http_program(&format!(
        r#"(
	i/fail
	std/http
)
main = () {{
	result = fail.catch(() {{ http.get("http://127.0.0.1:{port}/") }})
	result |>
		(Error(http.Connect(_))) {{ Nil }}
		| (_) {{ panic("expected connection error") }}
}}
"#
    ));
}

#[test]
fn http_redirect_methods() {
    for status in [301, 302, 303, 307, 308] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            for hop in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                if hop == 0 {
                    assert!(first.starts_with("POST /start "));
                    write!(stream, "HTTP/1.1 {status} Redirect\r\nLocation: /finish\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                } else {
                    if status == 307 || status == 308 {
                        assert!(first.starts_with("POST /finish "));
                        assert_eq!(body, b"hello");
                    } else {
                        assert!(first.starts_with("GET /finish "));
                        assert!(body.is_empty());
                    }
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .unwrap();
                }
            }
        });
        run_http_program(&format!(
            r#"
(std/http)

main = () {{ assert(http.post("http://127.0.0.1:{port}/start", <<"hello":utf8>>).status == http.Status(200)) }}
"#
        ));
        server.join().unwrap();
    }
}

#[test]
fn http_redirect_limit() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        for _ in 0..11 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                assert!(!line.is_empty());
            }
            stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: /again\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        }
    });
    run_http_program(&format!(
        r#"(
	i/fail
	std/http
)
main = () {{
	result = fail.catch(() {{ http.get("http://127.0.0.1:{port}/start") }})
	result |>
		(Error(http.Protocol("too many redirects"))) {{ Nil }}
		| (_) {{ panic("expected redirect limit") }}
}}
"#
    ));
    server.join().unwrap();
}

#[test]
fn http_redirect_drops_sensitive_headers() {
    let destination = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dest_port = destination.local_addr().unwrap().port();
    let origin = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = origin.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = origin.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            assert!(!line.is_empty());
        }
        write!(stream, "HTTP/1.1 302 Found\r\nLocation: http://localhost:{dest_port}/target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        let (mut stream, _) = destination.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut head = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            assert!(!line.is_empty());
            head.push_str(&line.to_ascii_lowercase());
        }
        assert!(head.starts_with("get /target "));
        assert!(!head.contains("authorization:"));
        assert!(!head.contains("cookie:"));
        assert!(!head.contains("proxy-authorization:"));
        assert!(head.contains("x-test: retained"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .unwrap();
    });
    run_http_program(&format!(
        r#"(
	std/http
)

main = () {{
	req = http.Request(
		method = http.Method("GET"),
		target = "http://127.0.0.1:{port}/",
		headers = http.Headers(
			[
				("Authorization", "secret"),
				("Cookie", "secret"),
				("Proxy-Authorization", "secret"),
				("X-Test", "retained"),
			],
		),
		body = <<>>,
		trailers = http.Headers([]),
	)
	assert(http.send(req).status == http.Status(200))
}}
"#
    ));
    server.join().unwrap();
}

#[test]
fn http_reuse() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut accepts = 1;
        for request_number in 0..2 {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                let (next, _) = listener.accept().unwrap();
                next.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                stream = next;
                reader = BufReader::new(stream.try_clone().unwrap());
                accepts += 1;
                reader.read_line(&mut line).unwrap();
            }
            assert!(line.starts_with("GET /"));
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                assert!(!line.is_empty());
            }
            let connection = if request_number == 1 {
                "close"
            } else {
                "keep-alive"
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: {connection}\r\n\r\n"
            )
            .unwrap();
        }
        accepts
    });
    run_http_program(&format!(
        r#"
(std/http)

main = () {{
 client = http.client()
 req = http.request(http.Method("GET"), "http://127.0.0.1:{port}/")
 assert(http.send_with(client.send, req).status == http.Status(200))
 assert(http.send_with(client.send, req).status == http.Status(200))
}}
"#
    ));
    assert_eq!(server.join().unwrap(), 1, "client did not reuse connection");
}

#[test]
fn http_reuse_replaces_closed_connection() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        for body in ["first", "second"] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(20)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
            }
            // Deliberately close without advertising it: the cached socket is stale.
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    run_http_program(&format!(
        r#"
(std/http)

main = () {{
 client = http.client()
 req = http.request(http.Method("GET"), "http://127.0.0.1:{port}/")
 assert(http.send_with(client.send, req).body == <<"first":utf8>>)
 assert(http.send_with(client.send, req).body == <<"second":utf8>>)
 client.close()
}}
"#
    ));
    server.join().unwrap();
}

#[test]
fn http_stream_request_body() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    let std_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    std::fs::write(
        project.path().join("i.toml"),
        format!(
            "name = \"http_stream_test\"\n[dependencies]\nstd = {{ path = {:?} }}\n",
            std_path.to_str().unwrap()
        ),
    )
    .unwrap();
    std::fs::write(
        project.path().join("src/main.i"),
        r#"(
	i/net
	i/task
	i/stdio
	i/core
	std/http
	std/bit_array
	std/list
	std/option
)
read_pieces = (
	body : (read = _, trailers = _),
	acc : List(BitArray),
	count : Int
) {
	piece = body.read(4096)
	{ bit_array.byte_size(piece) == 0 } |>
		(True) { (bit_array.concat(list.reverse(acc)), count) }
		| (False) {
			{ count == 0 } |> (True) { stdio.print("piece") } | (False) { Nil }
			read_pieces(body, [piece, ..acc], count + 1)
		}
}
handle = (req : http.Request, body : (read = _, trailers = _)) {
	{ req.target == "/ignore" || req.target == "/next" } |>
		(True) { http.response(http.Status(204)) }
		| (False) { echo_upload(req, body) }
}
echo_upload = (req : http.Request, body : (read = _, trailers = _)) {
	(bytes, count) = read_pieces(body, [], 0)
	assert(bit_array.byte_size(bytes) == 1048576)
	assert(count >= 256)
	{ req.target == "/chunked" } |>
		(True) {
			assert(
				http.header(body.trailers(), "X-End") == option.Some("yes"),
			)
		}
		| (False) { assert(body.trailers() == http.Headers([])) }
	res = http.response(http.Status(200))
	http.Response(..res, body = bytes)
}

main = () {
	listener = net.listen("127.0.0.1", 0)
	stdio.print(
		"listening " <> core.inspect(net.local_port(listener)),
	)
	http.serve_stream(listener, handle)
}
"#,
    )
    .unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--project"])
        .arg(project.path())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut server = Server(child);
    let mut stdout = BufReader::new(server.0.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let port: u16 = line
        .strip_prefix("listening ")
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let (pieces_tx, pieces_rx) = std::sync::mpsc::channel();
    let output_reader = thread::spawn(move || {
        for line in stdout.lines() {
            pieces_tx.send(line.unwrap()).unwrap();
        }
    });
    let body: Vec<u8> = (0..1048576).map(|n| (n % 251) as u8).collect();
    for chunked in [true, false] {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        if chunked {
            stream.write_all(b"POST /chunked HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nTransfer-Encoding: chunked\r\n\r\n1000\r\n").unwrap();
        } else {
            stream.write_all(b"POST /length HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 1048576\r\n\r\n").unwrap();
        }
        stream.write_all(&body[..4096]).unwrap();
        // The handler must consume a piece before the rest of the upload exists.
        assert_eq!(
            pieces_rx.recv_timeout(Duration::from_secs(30)).unwrap(),
            "piece"
        );
        if chunked {
            stream.write_all(b"\r\n").unwrap();
            for piece in body[4096..].chunks(8192) {
                write!(stream, "{:x}\r\n", piece.len()).unwrap();
                stream.write_all(piece).unwrap();
                stream.write_all(b"\r\n").unwrap();
            }
            stream.write_all(b"0\r\nX-End: yes\r\n\r\n").unwrap();
        } else {
            stream.write_all(&body[4096..]).unwrap();
        }
        let mut reply = Vec::new();
        stream.read_to_end(&mut reply).unwrap();
        assert!(
            reply.starts_with(b"HTTP/1.1 200 OK\r\n"),
            "{}",
            String::from_utf8_lossy(&reply[..reply.len().min(256)])
        );
        let head_end = reply.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        assert_eq!(&reply[head_end..], body);
    }
    // Leaving a body unread must drain its framing without consuming the next head.
    for framing in [
        "Transfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n0\r\nX-End: yes\r\n\r\n",
        "Content-Length: 3\r\n\r\nabc",
    ] {
        let pipeline = format!(
            "POST /ignore HTTP/1.1\r\nHost: localhost\r\n{framing}GET /next HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        );
        assert_eq!(
            request(port, pipeline.as_bytes())
                .matches("HTTP/1.1 204")
                .count(),
            2
        );
    }
    drop(server);
    output_reader.join().unwrap();
}

#[test]
fn http_reuse_keeps_origins_separate() {
    let first = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let second = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let first_port = first.local_addr().unwrap().port();
    let second_port = second.local_addr().unwrap().port();
    let servers: Vec<_> = [(first, "first", 2), (second, "second", 1)]
        .into_iter()
        .map(|(listener, body, count)| thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            for _ in 0..count {
                loop {
                    let mut line = String::new();
                    assert!(reader.read_line(&mut line).unwrap() > 0);
                    if line == "\r\n" { break; }
                }
                // Reuse is allowed only after the final chunk and trailers are read.
                write!(stream, "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{body}\r\n0\r\nX-End: yes\r\n\r\n", body.len()).unwrap();
            }
        }))
        .collect();
    run_http_program(&format!(
        r#"
(std/http, std/option)

main = () {{
 client = http.client()
 first = http.request(http.Method("GET"), "http://127.0.0.1:{first_port}/")
 second = http.request(http.Method("GET"), "http://127.0.0.1:{second_port}/")
 a = http.send_with(client.send, first)
 assert(a.body == <<"first":utf8>>)
 assert(http.header(a.trailers, "X-End") == option.Some("yes"))
 assert(http.send_with(client.send, second).body == <<"second":utf8>>)
 assert(http.send_with(client.send, first).body == <<"first":utf8>>)
 client.close()
}}
"#
    ));
    for server in servers {
        server.join().unwrap();
    }
}
