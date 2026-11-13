//! Native multicore workers (multicore.md D1/D8/P1, native.md N5/N7/N8/N12).
//! The worker count comes from `I_WORKERS`; `I_WORKERS_TRACE` makes the host
//! print `i: worker W started a task` when a worker other than 0 starts a
//! task, and at the end of a run that published records
//! `i: workers N published P started [..] steps [..]` (tasks started and
//! steps run per worker; worker 0 is the entry's instance).
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const SPIN: &str = "spin = (n: Int, x: Int) -> Int {
	{ n == 0 } |>
		(True) { x }
		| (False) { spin(n - 1, { x * 75 + 74 } % 65537) }
}
";

fn project(name: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ilang-multicore-{name}-{}", std::process::id()));
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

/// The counts of the trace line.
struct Trace {
    published: u64,
    started: Vec<u64>,
    steps: Vec<u64>,
}

fn counts(text: &str) -> Vec<u64> {
    text.split('[')
        .nth(1)
        .unwrap()
        .split(']')
        .next()
        .unwrap()
        .split(", ")
        .map(|n| n.trim().parse().unwrap())
        .collect()
}

impl Run {
    fn trace(&self) -> Option<Trace> {
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
        let started = counts(line.split(" started ").nth(1)?);
        let steps = counts(line.split(" steps ").nth(1)?);
        Some(Trace {
            published,
            started,
            steps,
        })
    }
}

impl Trace {
    fn workers_that_started(&self) -> usize {
        self.started.iter().filter(|n| **n > 0).count()
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

/// Run on one and on eight workers: the same output; the eight-worker trace.
fn agree(name: &str, source: &str) -> (Run, Run, Trace) {
    let dir = project(name, source);
    let one = run(&dir, 1);
    let eight = run(&dir, 8);
    assert_eq!(one.stdout, eight.stdout, "{}", eight.stderr);
    assert!(
        one.trace().is_none(),
        "one worker publishes nothing: {}",
        one.stderr
    );
    let trace = eight
        .trace()
        .unwrap_or_else(|| panic!("no trace: {}", eight.stderr));
    eprintln!(
        "multicore {name}: 1 worker {:.2}s, 8 workers {:.2}s, started {:?}, load {}",
        one.seconds,
        eight.seconds,
        trace.started,
        load()
    );
    (one, eight, trace)
}

/// Every published record starts exactly once: no compute phase runs twice.
fn started_once(trace: &Trace) {
    assert_eq!(trace.started.iter().sum::<u64>(), trace.published);
}

/// Eight pure tasks: the same results, paths and random draws.
#[test]
fn cpu_bound_tasks_agree_across_worker_counts() {
    let (_, _, trace) = agree(
        "cpu",
        &format!(
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
        ),
    );
    assert_eq!(trace.published, 8);
    started_once(&trace);
    assert!(trace.workers_that_started() > 1);
}

/// Acceptance 1: compute, then yield and a channel send, then compute.
#[test]
fn tasks_keep_running_on_their_worker_across_steps() {
    let (_, _, trace) = agree(
        "steps",
        &format!(
            "(
	i/task
	i/stdio
	i/core
)

{SPIN}
collect = (c: task.Channel(Int), n: Int, acc: Int) -> Int {{
	{{ n == 0 }} |>
		(True) {{ acc }}
		| (False) {{ collect(c, n - 1, acc + task.receive(c)) }}
}}

main = () {{
	(results, total) = task.scope((s) {{
		c: task.Channel(Int) = task.channel()
		sum = task.go(s, () {{ collect(c, 8, 0) }})
		results = task.map_concurrently([1, 2, 3, 4, 5, 6, 7, 8], (i) {{
			a = spin(2000000 + i, i)
			task.yield()
			task.send(c, a)
			(spin(2000000 + i, a), task.path())
		}})
		(results, task.join(sum))
	}})
	stdio.print(core.inspect(results))
	stdio.print(core.inspect(total))
}}
"
        ),
    );
    started_once(&trace);
    assert!(trace.workers_that_started() > 1, "{:?}", trace.started);
    // A remote task took its steps on its own worker.
    assert!(
        trace.steps.iter().skip(1).any(|n| *n >= 4),
        "{:?}",
        trace.steps
    );
}

/// Acceptance 2: a task on a worker spawns eight children.
#[test]
fn nested_go_is_parallel() {
    let (_, _, trace) = agree(
        "nested",
        &format!(
            "(
	i/task
	i/stdio
	i/core
)

{SPIN}
main = () {{
	results = task.scope((s) {{
		parent = task.go(s, () {{
			task.map_concurrently([1, 2, 3, 4, 5, 6, 7, 8], (i) {{ (spin(2000000 + i, i), task.path()) }})
		}})
		task.join(parent)
	}})
	stdio.print(core.inspect(results))
}}
"
        ),
    );
    assert_eq!(trace.published, 9);
    started_once(&trace);
    assert!(trace.workers_that_started() > 1, "{:?}", trace.started);
}

/// Acceptance 4: producer, four workers and a collector across workers.
#[test]
fn channel_pipeline_agrees_across_worker_counts() {
    let (_, _, trace) = agree(
        "pipeline",
        &format!(
            "(
	i/task
	i/stdio
	i/core
)

{SPIN}
produce = (out: task.Channel(Int), i: Int, n: Int) -> Nil {{
	{{ i > n }} |>
		(True) {{ Nil }}
		| (False) {{
			task.send(out, i)
			produce(out, i + 1, n)
		}}
}}

work = (
	input: task.Channel(Int),
	out: task.Channel((Int, Int)),
	n: Int
) -> Nil {{
	{{ n == 0 }} |>
		(True) {{ Nil }}
		| (False) {{
			i = task.receive(input)
			task.send(out, (i, spin(300000 + i, i)))
			work(input, out, n - 1)
		}}
}}

gather = (out: task.Channel((Int, Int)), n: Int, acc: List((Int, Int))) -> List(
	(Int, Int),
) {{
	{{ n == 0 }} |>
		(True) {{ acc }}
		| (False) {{ gather(out, n - 1, [task.receive(out), ..acc]) }}
}}

insert = (x: (Int, Int), xs: List((Int, Int))) -> List((Int, Int)) {{
	xs |>
		([]) {{ [x] }}
		| ([y, ..rest]) {{
			(a, _) = x
			(b, _) = y
			{{ a < b }} |>
				(True) {{ [x, y, ..rest] }}
				| (False) {{ [y, ..insert(x, rest)] }}
		}}
}}

sort = (xs: List((Int, Int)), acc: List((Int, Int))) -> List((Int, Int)) {{
	xs |> ([]) {{ acc }} | ([x, ..rest]) {{ sort(rest, insert(x, acc)) }}
}}

main = () {{
	results = task.scope((s) {{
		input: task.Channel(Int) = task.channel()
		out: task.Channel((Int, Int)) = task.channel()
		task.go(s, () {{ produce(input, 1, 32) }})
		task.go(s, () {{ work(input, out, 8) }})
		task.go(s, () {{ work(input, out, 8) }})
		task.go(s, () {{ work(input, out, 8) }})
		task.go(s, () {{ work(input, out, 8) }})
		collector = task.go(s, () {{ gather(out, 32, []) }})
		sort(task.join(collector), [])
	}})
	stdio.print(core.inspect(results))
}}
"
        ),
    );
    started_once(&trace);
    assert!(trace.workers_that_started() > 1, "{:?}", trace.started);
}

/// Acceptance 5: a failing child cancels tasks blocked in receive and in a
/// one-minute sleep on other workers; the run ends at once.
#[test]
fn cancel_reaches_blocked_tasks_on_other_workers() {
    let (_, eight, trace) = agree(
        "cancelio",
        &format!(
            "(
	i/task
	i/stdio
	i/core
	i/fail
	i/clock
)

{SPIN}
main = () {{
	outcome = fail.catch(() {{
		task.scope((s) {{
			c: task.Channel(Int) = task.channel()
			task.go(s, () {{ clock.sleep_ns(60000000000) }})
			task.go(s, () {{ clock.sleep_ns(60000000000) }})
			task.go(s, () {{ task.receive(c) }})
			task.go(s, () {{ task.receive(c) }})
			task.go(s, () {{
				spin(2000000, 3)
				fail.raise(\"stop\")
			}})
			Nil
		}})
	}})
	stdio.print(core.inspect(outcome))
}}
"
        ),
    );
    assert_eq!(eight.stdout.trim(), "Error(\"stop\")");
    assert!(eight.seconds < 50.0, "the sleepers were cancelled");
    assert!(
        trace.started.iter().skip(1).sum::<u64>() >= 1,
        "{:?}",
        trace.started
    );
    let (_, eight, trace) = agree(
        "cancelrecv",
        &format!(
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
			c: task.Channel(Int) = task.channel()
			task.go(s, () {{ task.receive(c) }})
			task.go(s, () {{ task.receive(c) }})
			task.go(s, () {{ task.receive(c) }})
			task.go(s, () {{
				spin(2000000, 3)
				fail.raise(\"stop\")
			}})
			Nil
		}})
	}})
	stdio.print(core.inspect(outcome))
}}
"
        ),
    );
    assert_eq!(eight.stdout.trim(), "Error(\"stop\")");
    assert!(
        trace.started.iter().skip(1).sum::<u64>() >= 1,
        "{:?}",
        trace.started
    );
}

/// Acceptance 6: records that hold a boundary or a local channel stay on
/// their maker and the result does not change.
#[test]
fn non_portable_records_stay_home() {
    let dir = project(
        "pinned",
        &format!(
            "(
	i/task
	i/stdio
	i/core
	i/control
)

{SPIN}
touch = <A>(_: A) -> Int {{ 0 }}

main = () {{
	results = control.boundary((b) {{
		task.map_concurrently([1, 2, 3, 4], (i) {{
			spin(200000 + i, i) + touch(b)
		}})
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
        ),
    );
    let one = run(&dir, 1);
    let eight = run(&dir, 8);
    assert_eq!(one.stdout, eight.stdout);
    assert!(
        eight.trace().is_none(),
        "no record was published: {}",
        eight.stderr
    );
}

/// `scope.share` values read on other workers.
#[test]
fn shared_values_are_read_on_other_workers() {
    let (_, _, trace) = agree(
        "share",
        &format!(
            "(
	i/task
	i/stdio
	i/core
)

{SPIN}
upto = (n: Int, acc: List(Int)) -> List(Int) {{
	{{ n == 0 }} |> (True) {{ acc }} | (False) {{ upto(n - 1, [n, ..acc]) }}
}}

total = (xs: List(Int), acc: Int) -> Int {{
	xs |> ([]) {{ acc }} | ([x, ..rest]) {{ total(rest, {{ acc + x }} % 65537) }}
}}

main = () {{
	results = task.scope((s) {{
		shared = task.share(s, upto(2000, []))
		task.map_concurrently([1, 2, 3, 4, 5, 6, 7, 8], (i) {{
			spin(
				1500000 + i,
				total(task.get(shared), i),
			)
		}})
	}})
	stdio.print(core.inspect(results))
}}
"
        ),
    );
    assert_eq!(trace.published, 8);
    started_once(&trace);
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn serve_clients(project: &Path, workers: Option<usize>, clients: usize) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_i"));
    command
        .args(["run", "--opt", "0", "--project"])
        .arg(project)
        .env("I_WORKERS_TRACE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match workers {
        Some(n) => command.env("I_WORKERS", n.to_string()),
        None => command.env("I_WORKERS", "auto"),
    };
    let mut server = Server(command.spawn().unwrap());
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
    let mut streams: Vec<TcpStream> = (0..clients).map(|_| connect()).collect();
    for (i, stream) in streams.iter_mut().enumerate() {
        stream.write_all(format!("client {i} ").as_bytes()).unwrap();
    }
    for stream in &mut streams {
        stream.write_all(b"done").unwrap();
        stream.shutdown(Shutdown::Write).unwrap();
    }
    for (i, stream) in streams.iter_mut().enumerate() {
        let mut got = Vec::new();
        stream.read_to_end(&mut got).unwrap();
        assert_eq!(got, format!("client {i} done").into_bytes());
    }
    let _ = server.0.kill();
    let _ = server.0.wait();
    let mut stderr = String::new();
    server
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    stderr
}

/// Acceptance 3: an echo server in the entry run serves eight simultaneous
/// connections, handled on more than one worker (with their own IO).
#[test]
fn echo_connections_are_served_on_several_workers() {
    let dir = project(
        "echo8",
        "(
	i/net
	i/task
	i/stdio
	i/core
	i/resource
)

echo = (stream) {
	bytes = net.read(stream, 65536)
	core.bit_size(bytes) |>
		(0) { Nil }
		| (_) {
			net.write(stream, bytes)
			echo(stream)
		}
}

serve = (listener, scope) {
	stream = net.accept(listener)
	task.go(scope, () {
		resource.ensure(() { echo(stream) }, () { net.close_stream(stream) })
	})
	serve(listener, scope)
}

// Program entry.
main = () {
	listener = net.listen(\"127.0.0.1\", 0)
	stdio.print(
		\"listening \" <> core.inspect(net.local_port(listener)),
	)
	task.scope((scope) { serve(listener, scope) })
}
",
    );
    let stderr = serve_clients(&dir, Some(8), 8);
    let mut workers: Vec<&str> = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("i: worker "))
        .filter_map(|rest| rest.strip_suffix(" started a task"))
        .collect();
    workers.sort_unstable();
    workers.dedup();
    assert!(workers.len() > 1, "{stderr}");
}

/// The echo example (a nested run, which stays one worker) still serves
/// three simultaneous connections with `I_WORKERS=auto`.
#[test]
fn echo_example_serves_three_connections_with_auto_workers() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/echo");
    serve_clients(&project, None, 3);
}
