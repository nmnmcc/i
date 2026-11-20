//! `i run | head -1`: when standard output's reader goes away, the running
//! program stops quietly with status 0, like the CLI's own output.

#[path = "common/test_project.rs"]
mod test_project;

use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const MAIN: &str = "(
	i/stdio
	i/clock
)

tick = (n : Int) -> () {
	stdio.print(\"tick\")
	clock.sleep_ns(20_000_000)
	tick(n + 1)
}

main = () {
	tick(0)
}
";

#[test]
fn run_stops_quietly_when_stdout_reader_goes_away() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        format!("name = \"pipe\"\n{}", test_project::stdlib_dependency()),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), MAIN).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--project"])
        .arg(dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let stderr = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    let mut line = String::new();
    {
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        stdout.read_line(&mut line).unwrap();
        // Dropping the reader closes the pipe.
    }
    if line != "tick\n" {
        let _ = child.kill();
        let _ = child.wait();
        panic!("first line {line:?}; stderr:\n{}", stderr.join().unwrap());
    }

    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "`i run` kept running after its reader went away; stderr:\n{}",
                stderr.join().unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stderr = stderr.join().unwrap();
    assert!(status.success(), "status {status}; stderr:\n{stderr}");
    assert_eq!(stderr, "", "stderr should be empty");
}
