use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn concurrent_echo_clients() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/echo");
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
    let connect = || {
        let addresses = ("localhost", port).to_socket_addrs().unwrap();
        let stream = addresses
            .filter_map(|addr| TcpStream::connect(addr).ok())
            .next()
            .unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
    };
    let mut a = connect();
    let mut b = connect();
    a.write_all(b"first ").unwrap();
    b.write_all(b"second ").unwrap();
    a.write_all(b"client").unwrap();
    b.write_all(b"connection").unwrap();
    a.shutdown(Shutdown::Write).unwrap();
    b.shutdown(Shutdown::Write).unwrap();
    let mut one = Vec::new();
    let mut two = Vec::new();
    a.read_to_end(&mut one).unwrap();
    b.read_to_end(&mut two).unwrap();
    assert_eq!(one, b"first client");
    assert_eq!(two, b"second connection");
}
