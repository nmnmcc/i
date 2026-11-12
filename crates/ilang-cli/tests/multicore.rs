//! Native multicore workers (multicore.md D1/D8/P1, native.md N5/N7/N12).
//! The worker count comes from `I_WORKERS`; `I_WORKERS_TRACE` makes the host
//! print, at the end of a run that published records, how many tasks each
//! worker ran and how many speculations a cancel stopped.
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const SPIN: &str = "spin = (n: Int, x: Int) -> Int {
	{ n == 0 } |> {
		True  -> x
		False -> spin(n - 1, { x * 75 + 74 } % 65537)
	}
}
";

fn project(name: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ilang-multicore-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("i.toml"), format!("name = {name:?}\n")).unwrap();
    std::fs::write(dir.join("src/main.i"), source).unwrap();
    dir
}

struct Run {
    stdout: String,
    stderr: String,
    seconds: f64,
}

impl Run {
    /// The counts of the trace line: (published, tasks per worker, stopped).
    fn trace(&self) -> Option<(u64, Vec<u64>, u64)> {
        let line = self
            .stderr
            .lines()
            .find(|line| line.starts_with("i: workers "))?;
        let published = line
            .split(" published ")
            .nth(1)?
            .split(' ')
            .next()?
            .parse()
            .ok()?;
        let ran = line.split('[').nth(1)?.split(']').next()?;
        let ran = ran
            .split(", ")
            .map(|n| n.trim().parse().unwrap())
            .collect();
        let stopped = line.split(" stopped ").nth(1)?.trim().parse().ok()?;
        Some((published, ran, stopped))
    }
}

fn run(dir: &Path, workers: usize) -> Run {
    let started = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--project"])
        .arg(dir)
        .env("I_WORKERS", workers.to_string())
        .env("I_WORKERS_TRACE", "1")
        .output()
        .unwrap();
    let seconds = started.elapsed().as_secs_f64();
    let run = Run {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        seconds,
    };
    assert!(
        out.status.success(),
        "i run failed with {workers} workers: {}",
        run.stderr
    );
    run
}

fn load() -> String {
    std::fs::read_to_string("/proc/loadavg").unwrap_or_default()
}

/// Acceptance 1: eight pure tasks print the same on one and eight workers,
/// and on eight workers more than one OS thread ran a task.
#[test]
fn cpu_bound_tasks_agree_across_worker_counts() {
    let source = format!(
        "(
	i/task
	i/stdio
	i/core
	i/random
)

{SPIN}
main = () {{
	results = task.map_concurrently([1, 2, 3, 4, 5, 6, 7, 8], (i) {{
		(draw, _) = random.int(task.random(), 1000000)
		(spin(3000000 + i, i), task.path(), draw)
	}})
	stdio.print(core.inspect(results))
}}
"
    );
    let dir = project("cpu", &source);
    let one = run(&dir, 1);
    let eight = run(&dir, 8);
    assert_eq!(one.stdout, eight.stdout);
    assert!(one.trace().is_none(), "one worker publishes nothing: {}", one.stderr);
    let (published, ran, _) = eight.trace().expect(&eight.stderr);
    assert_eq!(published, 8, "{}", eight.stderr);
    assert_eq!(ran.iter().sum::<u64>(), 8, "{}", eight.stderr);
    assert!(
        ran.iter().filter(|n| **n > 0).count() > 1,
        "more than one worker thread ran a task: {}",
        eight.stderr
    );
    eprintln!(
        "multicore timing: 1 worker {:.2}s, 8 workers {:.2}s, ratio {:.2}, load {}",
        one.seconds,
        eight.seconds,
        eight.seconds / one.seconds,
        load()
    );
}

/// Acceptance 2: records that hold a boundary or a local channel stay on
/// their maker (they are never published) and the result does not change.
#[test]
fn non_portable_records_stay_home() {
    let source = format!(
        "(
	i/task
	i/stdio
	i/core
	i/control
)

{SPIN}
touch(A) = (_: A) -> Int {{ 0 }}

main = () {{
	results = control.boundary((b) {{
		task.map_concurrently([1, 2, 3, 4], (i) {{ spin(200000 + i, i) + touch(b) }})
	}})
	stdio.print(core.inspect(results))
	local = task.scope((s) {{
		c: task.Channel(Int) = task.local_channel()
		task.go(s, () {{ task.send(c, spin(300000, 3)) }})
		task.receive(c)
	}})
	stdio.print(core.inspect(local))
}}
"
    );
    let dir = project("pinned", &source);
    let one = run(&dir, 1);
    let eight = run(&dir, 8);
    assert_eq!(one.stdout, eight.stdout);
    assert!(
        eight.trace().is_none(),
        "no record was published: {}",
        eight.stderr
    );
}

/// Acceptance 3: a failing child cancels its scope, and the cancel stops a
/// sibling that runs on another worker.
#[test]
fn cancel_stops_tasks_on_other_workers() {
    let source = format!(
        "(
	i/task
	i/stdio
	i/core
	i/fail
)

{SPIN}
main = () {{
	outcome = fail.catch(() {{
		task.scope((s) {{
			task.go(s, () {{ spin(6000000, 1) }})
			task.go(s, () {{ spin(4000000000, 2) }})
			task.go(s, () {{
				spin(1000000, 3)
				fail.raise(\"stop\")
			}})
			Nil
		}})
	}})
	stdio.print(core.inspect(outcome))
}}
"
    );
    let dir = project("cancel", &source);
    let eight = run(&dir, 8);
    assert_eq!(eight.stdout.trim(), "Error(\"stop\")");
    let (_, _, stopped) = eight.trace().expect(&eight.stderr);
    assert!(stopped >= 1, "a speculation was stopped: {}", eight.stderr);
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Acceptance 4: the echo example serves three simultaneous connections
/// with the default worker count.
#[test]
fn echo_serves_three_connections_with_default_workers() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/echo");
    let child = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--project"])
        .arg(project)
        .env_remove("I_WORKERS")
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
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        stream
    };
    let mut clients = [connect(), connect(), connect()];
    let words: [&[u8]; 3] = [b"first ", b"second ", b"third "];
    for (client, word) in clients.iter_mut().zip(words) {
        client.write_all(word).unwrap();
    }
    for client in &mut clients {
        client.write_all(b"client").unwrap();
        client.shutdown(Shutdown::Write).unwrap();
    }
    for (client, word) in clients.iter_mut().zip(words) {
        let mut got = Vec::new();
        client.read_to_end(&mut got).unwrap();
        assert_eq!(got, [word, b"client"].concat());
    }
}

/// `scope.share` keeps the value once in the host transport: tasks on other
/// workers read it without going back to their maker.
#[test]
fn shared_values_are_read_on_other_workers() {
    let source = format!(
        "(
	i/task
	i/stdio
	i/core
)

{SPIN}
upto = (n: Int, acc: List(Int)) -> List(Int) {{
	{{ n == 0 }} |> {{
		True  -> acc
		False -> upto(n - 1, [n, ..acc])
	}}
}}

total = (xs: List(Int), acc: Int) -> Int {{
	xs |> {{
		[]          -> acc
		[x, ..rest] -> total(rest, { acc + x } % 65537)
	}}
}}

main = () {{
	results = task.scope((s) {{
		shared = task.share(s, upto(2000, []))
		task.map_concurrently([1, 2, 3, 4, 5, 6, 7, 8], (i) {{
			spin(1500000 + i, total(task.get(shared), i))
		}})
	}})
	stdio.print(core.inspect(results))
}}
"
    );
    let dir = project("share", &source);
    let one = run(&dir, 1);
    let eight = run(&dir, 8);
    assert_eq!(one.stdout, eight.stdout);
    let (published, ran, _) = eight.trace().expect(&eight.stderr);
    assert_eq!(published, 8, "{}", eight.stderr);
    assert!(
        ran.iter().skip(1).any(|n| *n > 0),
        "a remote worker read the shared value: {}",
        eight.stderr
    );
}
