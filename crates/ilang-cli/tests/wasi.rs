//! The WASI target (spec B.7): `i build --target wasi` makes a WASI
//! preview 1 command. These tests run it in Wasmtime with a small
//! `fd_write` and `proc_exit` of their own, which may write only a few bytes
//! at a time, and link the other imports as a host would.

use ilang_cli::{BuildError, BuildOptions, Level, Target, build};
use ilang_host::{Abi, Outcome, Program, Stop};
use std::path::Path;
use wasmtime::{Caller, Config, Engine, Extern, Linker, Module, Store};

/// The import of the standard service (spec §9.7).
const IO: &str = " ((Io) = i/io)\n";

fn package(main: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), main).unwrap();
    dir
}

fn options(opt: Level, target: Target) -> BuildOptions {
    BuildOptions {
        tests: false,
        opt,
        offline: true,
        target,
    }
}

fn wasi(project: &Path, opt: Level) -> Vec<u8> {
    match build(project, options(opt, Target::Wasi)) {
        Ok(b) => b.wasm,
        Err(e) => panic!("{e}"),
    }
}

#[derive(Default)]
struct State {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit: Option<i32>,
    /// The most bytes that one `fd_write` writes.
    chunk: usize,
}

#[derive(Debug, PartialEq)]
struct Ran {
    stdout: String,
    stderr: String,
    /// The exit code; a trap is 134, as in the Wasmtime command.
    code: i32,
}

fn engine() -> Engine {
    let mut config = Config::new();
    config
        .wasm_gc(true)
        .wasm_function_references(true)
        .wasm_tail_call(true)
        .wasm_exceptions(true);
    Engine::new(&config).unwrap()
}

fn memory(caller: &mut Caller<'_, State>) -> wasmtime::Memory {
    match caller.get_export("memory") {
        Some(Extern::Memory(m)) => m,
        _ => panic!("no memory"),
    }
}

/// Runs `_start` of a WASI command. `link` adds the imports other than
/// WASI's.
fn run(wasm: &[u8], chunk: usize, link: impl FnOnce(&mut Linker<State>)) -> Ran {
    let engine = engine();
    let module = Module::new(&engine, wasm).unwrap();
    let mut linker: Linker<State> = Linker::new(&engine);
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "fd_write",
            |mut caller: Caller<'_, State>, fd: i32, iovs: i32, n: i32, written: i32| -> i32 {
                let memory = memory(&mut caller);
                let mut bytes = Vec::new();
                for k in 0..n as usize {
                    let mut iov = [0u8; 8];
                    memory
                        .read(&caller, iovs as usize + 8 * k, &mut iov)
                        .unwrap();
                    let address = u32::from_le_bytes(iov[..4].try_into().unwrap()) as usize;
                    let length = u32::from_le_bytes(iov[4..].try_into().unwrap()) as usize;
                    let mut part = vec![0; length];
                    memory.read(&caller, address, &mut part).unwrap();
                    bytes.extend(part);
                }
                bytes.truncate(caller.data().chunk);
                let count = bytes.len() as u32;
                match fd {
                    1 => caller.data_mut().stdout.extend(bytes),
                    2 => caller.data_mut().stderr.extend(bytes),
                    _ => return 8, // EBADF
                }
                memory
                    .write(&mut caller, written as usize, &count.to_le_bytes())
                    .unwrap();
                0
            },
        )
        .unwrap();
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "proc_exit",
            |mut caller: Caller<'_, State>, code: i32| -> wasmtime::Result<()> {
                caller.data_mut().exit = Some(code);
                wasmtime::bail!("exit {code}")
            },
        )
        .unwrap();
    link(&mut linker);
    let mut store = Store::new(
        &engine,
        State {
            chunk,
            ..State::default()
        },
    );
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .unwrap();
    let result = start.call(&mut store, ());
    let state = store.data();
    let code = match (result, state.exit) {
        (Ok(()), None) => 0,
        (_, Some(code)) => code,
        (Err(_), None) => 134,
    };
    Ran {
        stdout: String::from_utf8(state.stdout.clone()).unwrap(),
        stderr: String::from_utf8(state.stderr.clone()).unwrap(),
        code,
    }
}

fn no_imports(_: &mut Linker<State>) {}

/// The stop message that the ilang host gets for the same program.
fn ilang_stop(project: &Path) -> String {
    let b = build(project, options(Level::O0, Target::Native)).unwrap_or_else(|e| panic!("{e}"));
    let program = Program::from_bytes(&b.wasm).unwrap();
    match program.run_entry(Default::default()) {
        Ok(Outcome::Stop(stop)) => stop.message.unwrap(),
        // A stop in `ilang_start`.
        Err(e) => e.downcast::<Stop>().unwrap().message.unwrap(),
        other => panic!("not a stop: {other:?}"),
    }
}

fn imports(wasm: &[u8]) -> Vec<(String, String)> {
    let module = Module::new(&engine(), wasm).unwrap();
    module
        .imports()
        .map(|i| (i.module().to_string(), i.name().to_string()))
        .collect()
}

fn exports(wasm: &[u8]) -> Vec<String> {
    let module = Module::new(&engine(), wasm).unwrap();
    module.exports().map(|e| e.name().to_string()).collect()
}

const LEVELS: [Level; 2] = [Level::O0, Level::O2];

#[test]
fn hello_needs_only_wasi() {
    let dir = package(&format!(
        "{IO}\nmain = () <- (io: Io) {{ io.print(\"Hello from i!\") }}\n"
    ));
    for opt in LEVELS {
        let wasm = wasi(dir.path(), opt);
        assert_eq!(
            imports(&wasm),
            [
                ("wasi_snapshot_preview1".into(), "fd_write".into()),
                ("wasi_snapshot_preview1".into(), "proc_exit".into())
            ],
            "{opt:?}"
        );
        assert!(exports(&wasm).contains(&"_start".to_string()));
        let abi = Abi::from_wasm(&wasm).unwrap().unwrap();
        assert!(abi.services.is_empty(), "the module gives print");
        assert_eq!(abi.target.as_deref(), Some("wasi"));
        let refused = Program::from_bytes(&wasm).err().unwrap().to_string();
        assert!(refused.contains("wasmtime run"), "{refused}");
        assert_eq!(
            run(&wasm, usize::MAX, no_imports),
            Ran {
                stdout: "Hello from i!\n".into(),
                stderr: String::new(),
                code: 0
            }
        );
    }
}

#[test]
fn stops_write_the_message_and_exit_with_1() {
    let programs = [
        ("", "\nmain = () { assert(1 + 1 == 3) }\n"),
        // An error at the entry (§9.3).
        (
            " (i/fail)\n",
            "Bad = { (Int) }\n\nmain = () { fail.raise(Bad(7)) }\n",
        ),
        // A stop while the top-level values are worked out (§2.5).
        ("", "early = panic(\"too early\")\n\nmain = () { early }\n"),
    ];
    for (imports, source) in programs {
        let dir = package(&format!("{imports}{source}"));
        let message = ilang_stop(dir.path());
        for opt in LEVELS {
            let ran = run(&wasi(dir.path(), opt), 5, no_imports);
            assert_eq!(
                ran,
                Ran {
                    stdout: String::new(),
                    stderr: format!("{message}\n"),
                    code: 1
                },
                "{source} at {opt:?}"
            );
        }
    }
}

#[test]
fn long_output_survives_short_writes() {
    let dir = package(&format!(
        "{IO}
double = (s: String, n: Int) {{
  n
  |> (0) {{ s }}
  | (_) {{ double(s <> s, n - 1) }}
}}

lines = (n: Int) <- (io: Io) {{
  n
  |> (0) {{ Nil }}
  | (_) {{
    io.print(\"ünïcödé ✓\")
    lines(n - 1)
  }}
}}


main = () <- (io: Io) {{
  io.print(double(\"héllo ✓ \", 14))
  lines(1000)
}}
"
    ));
    let mut want = "héllo ✓ ".repeat(1 << 14);
    want.push('\n');
    want.push_str(&"ünïcödé ✓\n".repeat(1000));
    for opt in LEVELS {
        let ran = run(&wasi(dir.path(), opt), 7, no_imports);
        assert_eq!(ran.code, 0, "{}", ran.stderr);
        assert!(ran.stdout == want, "{opt:?}: the output is not the same");
    }
}

/// A provider of `app:greet/v1` `greet: (name: String) -> String`, as in
/// B.2 and B.3: it reads the parameter and gives a buffer of `ilang_alloc`.
fn greeter(linker: &mut Linker<State>) {
    linker
        .func_wrap(
            "app:greet/v1",
            "greet",
            |mut caller: Caller<'_, State>, address: i32, length: i32| -> (i32, i32) {
                let memory = memory(&mut caller);
                let mut name = vec![0; length as usize];
                memory.read(&caller, address as usize, &mut name).unwrap();
                let text = format!("Hello, {}!", String::from_utf8(name).unwrap());
                let alloc = caller
                    .get_export("ilang_alloc")
                    .and_then(Extern::into_func)
                    .unwrap()
                    .typed::<i32, i32>(&caller)
                    .unwrap();
                let buffer = alloc.call(&mut caller, text.len() as i32).unwrap();
                memory
                    .write(&mut caller, buffer as usize, text.as_bytes())
                    .unwrap();
                (buffer, text.len() as i32)
            },
        )
        .unwrap();
}

fn greet_program(attribute: &str) -> tempfile::TempDir {
    package(&format!(
        "{IO}{attribute}\nGreet = (greet = (String) -> String)\n\
         \nmain = () <- (io: Io, g: Greet) {{ io.print(g.greet(\"i\")) }}\n"
    ))
}

#[test]
fn other_services_stay_imports_of_the_binary_interface() {
    let dir = greet_program("@host(\"app:greet/v1\")");
    for opt in LEVELS {
        let wasm = wasi(dir.path(), opt);
        assert!(imports(&wasm).contains(&("app:greet/v1".into(), "greet".into())));
        assert_eq!(run(&wasm, usize::MAX, greeter).stdout, "Hello, i!\n");
    }
}

#[test]
fn async_services_are_blocking_calls() {
    let dir = greet_program("@host(\"app:greet/v1\", \"async\")");
    for opt in LEVELS {
        let wasm = wasi(dir.path(), opt);
        let abi = Abi::from_wasm(&wasm).unwrap().unwrap();
        assert!(!abi.entry.as_ref().unwrap().is_async);
        assert!(!abi.services["app:greet/v1"]["greet"].is_async);
        assert!(
            !exports(&wasm)
                .iter()
                .any(|e| e.starts_with("ilang_resume:") || e.starts_with("ilang_result:"))
        );
        assert_eq!(run(&wasm, usize::MAX, greeter).stdout, "Hello, i!\n");
    }
}

#[test]
fn host_references_keep_the_reference_id_import() {
    let dir = package(
        "Handle = Opaque\n\
         @host(\"app:files/v1\")\n\
         Files = (open = () -> Handle, same = (Handle, Handle) -> Bool)\n\
         \n\
         main = () <- (f: Files) { f.open() == f.open() }\n\
         ",
    );
    let wasm = wasi(dir.path(), Level::O0);
    assert!(imports(&wasm).contains(&("ilang:host/v1".into(), "reference_id".into())));
    // Without host references, `reference_id` is defined in the module.
    let dir = package("\nmain = () { Nil }\n");
    let wasm = wasi(dir.path(), Level::O0);
    assert!(
        !imports(&wasm).iter().any(|(m, _)| m == "ilang:host/v1"),
        "{:?}",
        imports(&wasm)
    );
}

fn build_error(source: &str) -> String {
    let dir = package(source);
    match build(dir.path(), options(Level::O0, Target::Wasi)) {
        Err(e @ BuildError::Program { .. }) => e.to_string(),
        Err(e) => panic!("{e}"),
        Ok(_) => panic!("built"),
    }
}

#[test]
fn needs_an_entry() {
    assert!(build_error("f = () { 1 }\n").contains("needs a program entry"));
}

/// The standard service is `Io` of `i/io`. Another declaration of `print`
/// in `i:io/v1` is two host services with one field (E0311); one with
/// another field is a normal host service, and stays an import.
#[test]
fn the_standard_service_is_the_one_of_i_io() {
    let twice = "@host(\"i:io/v1\")\n\
                 Mine = (print = (String) -> Nil)\n\
                 \n\
                 main = () <- (io: Mine) { io.print(\"x\") }\n\
                 ";
    let e = build_error(twice);
    assert!(
        e.contains("E0311") && e.contains("the host service `Io` of `i/io`"),
        "{e}"
    );
    let other = "@host(\"i:io/v1\")\n\
                 Input = (read = () -> String)\n\
                 \n\
                 main = () <- (input: Input) { input.read() }\n\
                 ";
    let dir = package(other);
    let wasm = wasi(dir.path(), Level::O0);
    assert!(imports(&wasm).contains(&("i:io/v1".into(), "read".into())));
}

/// When an external function of the program and `Io` of `i/io` do not
/// agree about the import `i:io/v1` `print`, the error is told at the
/// declaration in the program, not in the library.
#[test]
fn an_import_that_does_not_agree_with_the_library_is_told_in_the_program() {
    let source = "(\n\t(Io) = i/io\n)\n@external(\"i:io/v1\", \"print\")\np: (String) -> Nil\n\nmain = () <- (io: Io) {\n\tio.print(\"a\")\n\tp(\"b\")\n}\n";
    for target in [Target::Wasi, Target::Native] {
        let dir = package(source);
        let e = match build(dir.path(), options(Level::O0, target)) {
            Err(e @ BuildError::Program { .. }) => e.to_string(),
            Err(e) => panic!("{e}"),
            Ok(_) => panic!("built"),
        };
        assert!(e.contains("src/main.i:2:31: error[E0701]"), "{e}");
    }
}

/// The runtime imports `fd_write` and `proc_exit` itself (B.7), so a
/// program may not import them, as an external function or as a field of
/// a host service. Other WASI functions are imports like any other.
#[test]
fn the_wasi_imports_of_the_runtime_are_refused() {
    let wasi_module = "wasi_snapshot_preview1";
    for name in ["fd_write", "proc_exit"] {
        let external = format!(
            "@external(\"{wasi_module}\", \"{name}\")\nf: (String) -> Nil\n\
             \nmain = () {{ f(\"x\") }}\n"
        );
        let e = build_error(&external);
        assert!(e.contains("the runtime imports"), "{name}: {e}");
        let service = format!(
            "@host(\"{wasi_module}\")\nW = ({name} = (Bool) -> Nil)\n\
             \nmain = () <- (w: W) {{ w.{name}(True) }}\n"
        );
        let e = build_error(&service);
        assert!(e.contains("the runtime imports"), "{name}: {e}");
    }
    let other = format!(
        "@external(\"{wasi_module}\", \"sched_yield\")\nyield: () -> Int\n\
         \nmain = () {{ yield() }}\n"
    );
    let dir = package(&other);
    let wasm = wasi(dir.path(), Level::O0);
    assert!(imports(&wasm).contains(&(wasi_module.into(), "sched_yield".into())));
    // On the target of the binary interface they are normal imports.
    let dir = package(&format!(
        "@external(\"{wasi_module}\", \"proc_exit\")\nexit: (Int) -> Nil\n\
         \nmain = () {{ exit(0) }}\n"
    ));
    assert!(build(dir.path(), options(Level::O0, Target::Native)).is_ok());
}

#[test]
fn only_wasi_modules_name_their_target() {
    let dir = package("\nmain = () { Nil }\n");
    let ilang = build(dir.path(), options(Level::O0, Target::Native))
        .unwrap()
        .wasm;
    assert_eq!(Abi::from_wasm(&ilang).unwrap().unwrap().target, None);
    let wasm = wasi(dir.path(), Level::O0);
    assert_eq!(
        Abi::from_wasm(&wasm).unwrap().unwrap().target.as_deref(),
        Some("wasi")
    );
}

// Lazy re-entry of paused frames: the programs of run tests 12 to 16 of
// docs/design/lazy-reentry.md (13.2), and an error through a final record
// (13.3), at `--opt 2`, with what eager at `1df2fe0` gives for them. The spec
// does not state these outputs (D15, D16).
// `crates/ilang-wasm/tests/control_lazy.rs` runs the same programs at
// `--opt 0`.

/// The start of the programs of run tests 12, 13 and 15. A cancel runs a
/// final function that suspends to `b2`, and `b0` takes the continuation
/// k2 of that suspend out of the cancel. Every computation that `put` runs
/// in needs `Out`, so `main` gives a placeholder `Out` first.
const CANCEL_OUT: &str = r#"(
	i/control
	i/fail
	i/resource
)

// Hands k2 out of the cancel, to the boundary `b0` around it.
Out = (put = (Inner) -> Nil)
Inner = (Continuation(Nil, Int, Never, Out))
Paused = (Continuation(Nil, Outer, Never, Out))
Done = ()
Outer = Paused | Done
Handed = (Inner)
Lost = ()
Handed = Handed | Lost

put = (k2) <- (o: Out) { o.put(k2) }
"#;

/// `g` of test 12: its frame waits for the ensure, so its record is pending.
const G_PLAIN: &str = r#"// A code frame with a site: it waits for the ensure, so its record is pending.
g = (b, b2) {
  x = resource.ensure(
    () {
      control.suspend(b, (k) { Paused(k) })
      0
    },
    () {
      control.suspend(b2, (k2) {
        put(Inner(k2))
        0
      })
    },
  )
  x + 1
}

"#;

/// `g` of the second part of test 12: a `fail.catch` that the cancel passes
/// over lies between the final record and the record of `g`.
const G_CATCH: &str = r#"// A code frame with a site, and a `fail.catch` that the cancel passes over.
g = (b, b2) {
	x = fail.catch(() {
		resource.ensure(
			() {
				control.suspend(b, (k) { Paused(k) })
				0
			},
			() {
				control.suspend(b2, (k2) {
					put(Inner(k2))
					0
				})
			},
		)
	})
	x |> (Ok(n)) { n + 1 }
}
"#;

/// Gives k2.
const PAUSED: &str = r#"// The cancel of `k` runs the final function, which suspends to `b2`; `b0`
// takes the continuation k2 of that suspend out of the cancel.
paused = () {
	k = {
		control.boundary((b) {
			control.boundary((b2) { g(b, b2) })
			Done
		})
	} |>
		(Paused(k)) { k }
		| (Done) { panic("not paused") }
	handed = control.boundary((b0) {
		{
			control.cancel(k)
			Lost
		} <- (Out(print = (k2) { control.suspend(b0, (rest) { Handed(k2) }) }))
	})
	handed |>
		(Handed(Inner(k2))) { k2 }
		| (Lost) { panic("k2 was not handed out") }
}
"#;

/// Test 12: the resume of k2 runs the end of the final function, which
/// throws the `$cancel` of `k`. Nothing takes it, and the entry stops.
const MAIN_OUT: &str = r#"// The resumed final function ends, and the `$cancel` of `k` goes out of
// the resume, which nothing takes.
main = () {
	// The computations need `Out`. This one is never called.
	{
		control.resume(paused(), Nil)
		Nil
	} <- (Out(print = (k2) { Nil }))
}
"#;

/// Test 13: k2 is resumed from frames that a resume of k3 entered.
const THIRD: &str = r#"Third = (Continuation(Nil, Third, Never, Out))
Over = ()
// `h` resumes k2, which needs `Out`, so k3 needs it too.
Third = Third | Over

keep = (t) { t |> (Over) { Over } | (t) { t } }

// Not in tail position, so the frame waits.
inner = (k2) { control.resume(k2, Nil) + 1 }

h = (b3, k2) {
	control.suspend(b3, (k3) { Third(k3) })
	inner(k2)
	Over
}
"#;

/// Test 13: the outer frame of k3 waits.
const O1_PLAIN: &str = r#"// The outer frame of k3, which waits.
o1 = (b3, k2) { keep(h(b3, k2)) }

"#;

/// Test 15: the ensure of k3 waits outside `h`.
const O1_ENSURE: &str = r#"// The ensure of k3 waits outside `h`: the cancel runs its final function.
o1 = (b3, k2) { keep(resource.ensure(() { h(b3, k2) }, () { panic("final") })) }

"#;

/// The entry of tests 13 and 15.
const MAIN_THIRD: &str = r#"// k2 is resumed from frames that a resume of k3 entered, while the outer
// frame of k3 waits.
main = () {
	// The computations need `Out`. This one is never called.
	{
		k2 = paused()
		k3 = { control.boundary((b3) { o1(b3, k2) }) } |>
			(Third(k3)) { k3 }
			| (Over) { panic("not paused") }
		control.resume(k3, Nil)
		Nil
	} <- (Out(print = (k2) { Nil }))
}
"#;

/// Test 14, resume: the tail call of `h_tail` leaves no frame that keeps
/// `! Name = Name("in")`, so `g` sees "resumer", against §9.4 rule 3
/// (section 7a).
const ENV_TAIL_RESUME: &str = r#"(
	i/control
)

Name = (text = String)
Paused = (Continuation(Nil, Paused, Never, Name))
Done = ()
Paused = Paused | Done

name = () <- (n: Name) { n.text }

g = (b) {
	control.suspend(b, (k) { Paused(k) })
	panic(name())
}

// The call of `g` is a tail call, so no frame keeps the environment
// statement when `g` is paused (section 7a of docs/design/lazy-reentry.md).
h_tail = (b) { g(b) <- (Name(text = "in")) }

main = () {
	{
		k = {
			control.boundary((b) {
				h_tail(b)
				Done
			})
		} |>
			(Paused(k)) { k }
			| (Done) { panic("not paused") }
		{
			control.resume(k, Nil)
			Nil
		} <- (Name(text = "resumer"))
	} <- (Name(text = "start"))
}
"#;

/// Test 14, cancel: the same for the final function of a cancel.
const ENV_TAIL_CANCEL: &str = r#"(
	i/control
	i/resource
)

Name = (text = String)
Paused = (Continuation(Nil, Paused, Never, Name))
Done = ()
Paused = Paused | Done

name = () <- (n: Name) { n.text }

g = (b) {
	resource.ensure(
		() {
			control.suspend(b, (k) { Paused(k) })
			Nil
		},
		() { panic(name()) },
	)
}

// The call of `g` is a tail call, so no frame keeps the environment
// statement when `g` is paused (section 7a of docs/design/lazy-reentry.md).
h_tail = (b) { g(b) <- (Name(text = "in")) }

main = () {
	{
		k = {
			control.boundary((b) {
				h_tail(b)
				Done
			})
		} |>
			(Paused(k)) { k }
			| (Done) { panic("not paused") }
		{
			control.cancel(k)
			Nil
		} <- (Name(text = "resumer"))
	} <- (Name(text = "start"))
}
"#;

/// Test 16: the value of `f` of `inner` goes on into the cancelled code.
const CANCEL_INNER: &str = r#"(
	i/control
	i/resource
	(Io) = i/io
)

Paused = (Continuation(Nil, Paused, Never, Io))
Done = ()
Paused = Paused | Done

say = (text) <- (io: Io) { io.print(text) }

// During the cancel, the final function suspends to `inner`, which is
// paused inside `k`; the value of its `f` goes on into the code after it.
body = (outer) {
	x = control.boundary((inner) {
		resource.ensure(
			() {
				control.suspend(outer, (k) { Paused(k) })
				"body"
			},
			() {
				say("final")
				y = control.suspend(inner, (k2) { "f of inner" })
				say("final again: " <> y)
			},
		)
	})
	say("after inner: " <> x)
	Done
}

main = () <- (io: Io) {
	k = { control.boundary((outer) { body(outer) }) } |>
		(Paused(k)) { k }
		| (Done) { panic("not paused") }
	say("cancel")
	control.cancel(k)
	say("cancel gave Nil")
}
"#;

/// An error goes through a pending final record (kind 5, skipped in mode
/// 1). A final function cannot raise an error itself (§9.5), but D15's
/// cancel through an inner boundary can run ordinary code that raises one.
/// This is checked against eager, rather than made a conformance rule.
const ERROR_THROUGH_FINAL: &str = r#"(
	i/control
	i/core
	i/fail
	i/resource
	(Io) = i/io
)

Paused = (Continuation(Nil, Paused, Int, Never))
Done = ()
Paused = Paused | Done
Held = (Continuation(Nil, Held, Never, Never))
Over = ()
Held = Held | Over

// Cancelling this continuation can return to the code after `inner`,
// because the final function suspends to `inner` during the cancel (D15).
paused = () {
	{
		control.boundary((outer) {
			control.boundary((inner) {
				resource.ensure(
					() {
						control.suspend(outer, (k) { Paused(k) })
						Nil
					},
					() { control.suspend(inner, (k2) { Nil }) },
				)
			})
			fail.raise(7)
			Done
		})
	} |>
		(Paused(k)) { k }
		| (Done) { panic("not paused") }
}

// `cancel` has no errors in its type. Here it runs the ordinary code
// after `inner`, whose error passes through this final function's record.
with = (hold, k) {
	control.suspend(hold, (k2) { Held(k2) })
	control.cancel(k)
}

main = () <- (io: Io) {
	k = paused()
	r = fail.catch(() {
		held = control.boundary((hold) {
			resource.ensure(() { 99 }, () { with(hold, k) })
			Over
		})
		held |>
			(Held(k2)) { control.resume(k2, Nil) }
			| (Over) { panic("not held") }
	})
	io.print(core.inspect(r))
}
"#;

/// Each program, and the standard output, the standard error and the exit
/// code that the eager compiler gives for it, at `--opt 0` and `--opt 2`.
fn eager(name: &str) -> (String, &'static str, &'static str, i32) {
    match name {
        "cancel_out_trace" => (
            [CANCEL_OUT, G_PLAIN, PAUSED, MAIN_OUT].concat(),
            "",
            "control: boundary is not active\napp/main:13:7\napp/main:55:3\n",
            1,
        ),
        "cancel_out_trace_catch" => (
            [CANCEL_OUT, G_CATCH, PAUSED, MAIN_OUT].concat(),
            "",
            "control: boundary is not active\napp/main:14:5\napp/main:13:7\napp/main:59:3\n",
            1,
        ),
        "cancel_out_two_trampolines" => (
            [CANCEL_OUT, G_PLAIN, PAUSED, THIRD, O1_PLAIN, MAIN_THIRD].concat(),
            "",
            "control: boundary is not active\napp/main:13:7\napp/main:60:16\napp/main:64:3\napp/main:69:22\napp/main:82:3\n",
            1,
        ),
        "cancel_out_restores_before_mode2" => (
            [CANCEL_OUT, G_PLAIN, PAUSED, THIRD, O1_ENSURE, MAIN_THIRD].concat(),
            "",
            "panic: final\napp/main:69:61\napp/main:69:22\napp/main:82:3\n",
            1,
        ),
        "env_tail_call_resume" => (
            ENV_TAIL_RESUME.to_string(),
            "",
            "panic: resumer\napp/main:10:3\napp/main:24:5\napp/main:31:3\n",
            1,
        ),
        "env_tail_call_cancel" => (
            ENV_TAIL_CANCEL.to_string(),
            "",
            "panic: resumer\napp/main:11:10\napp/main:26:5\napp/main:33:3\n",
            1,
        ),
        "cancel_suspends_to_inner_boundary" => (
            CANCEL_INNER.to_string(),
            "cancel\nfinal\nafter inner: f of inner\ncancel gave Nil\n",
            "",
            0,
        ),
        "error_through_pending_final" => (ERROR_THROUGH_FINAL.to_string(), "Error(7)\n", "", 0),
        _ => panic!("no program {name}"),
    }
}

/// Runs the program `name` at `--opt 2`, and checks that it gives what the
/// eager compiler gives.
fn lazy_same_as_eager(name: &str) {
    let (source, stdout, stderr, code) = eager(name);
    let dir = package(&source);
    assert_eq!(
        run(&wasi(dir.path(), Level::O2), usize::MAX, no_imports),
        Ran {
            stdout: stdout.into(),
            stderr: stderr.into(),
            code
        },
        "{name}"
    );
}

#[test]
fn lazy_cancel_out_trace() {
    lazy_same_as_eager("cancel_out_trace");
    lazy_same_as_eager("cancel_out_trace_catch");
}

#[test]
fn lazy_cancel_out_two_trampolines() {
    lazy_same_as_eager("cancel_out_two_trampolines");
}

#[test]
fn lazy_env_tail_call_as_eager() {
    lazy_same_as_eager("env_tail_call_resume");
    lazy_same_as_eager("env_tail_call_cancel");
}

#[test]
fn lazy_cancel_out_restores_before_mode2() {
    lazy_same_as_eager("cancel_out_restores_before_mode2");
}

#[test]
fn lazy_cancel_suspends_to_inner_boundary() {
    lazy_same_as_eager("cancel_suspends_to_inner_boundary");
}

#[test]
fn lazy_error_through_pending_final() {
    lazy_same_as_eager("error_through_pending_final");
}
