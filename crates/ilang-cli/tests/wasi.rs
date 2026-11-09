//! The WASI target (spec B.7): `i build --target wasi` makes a WASI
//! preview 1 command. These tests run it in Wasmtime with a small
//! `fd_write` and `proc_exit` of their own, which may write only a few bytes
//! at a time, and link the other imports as a host would.

use ilang_cli::{BuildError, BuildOptions, Level, Target, build};
use ilang_host::{Abi, Outcome, Program, Stop};
use std::path::Path;
use wasmtime::{Caller, Config, Engine, Extern, Linker, Module, Store};

/// The import of the standard service (spec §9.7).
const IO: &str = "import ((Io) = i/io)\n";

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
    let b = build(project, options(Level::O0, Target::Ilang)).unwrap_or_else(|e| panic!("{e}"));
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
        "{IO}@main\nmain = (io ! Io) {{ io.print(\"Hello from i!\") }}\n"
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
        ("", "@main\nmain = () { assert(1 + 1 == 3) }\n"),
        // An error at the entry (§9.3).
        (
            "import (i/fail)\n",
            "Bad { Bad(Int) }\n@main\nmain = () { fail.raise(Bad(7)) }\n",
        ),
        // A stop while the top-level values are worked out (§2.5).
        (
            "",
            "early = panic(\"too early\")\n@main\nmain = () { early }\n",
        ),
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
  case n {{
    0 -> s
    _ -> double(s <> s, n - 1)
  }}
}}

lines = (io ! Io, n: Int) {{
  case n {{
    0 -> Nil
    _ -> {{
      io.print(\"ünïcödé ✓\")
      lines(n - 1)
    }}
  }}
}}

@main
main = (io ! Io) {{
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
        "{IO}{attribute}\nGreet {{ Greet(greet: (name: String) -> String) }}\n\
         @main\nmain = (io ! Io, g ! Greet) {{ io.print(g.greet(\"i\")) }}\n"
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
        "Handle { }\n@host(\"app:files/v1\")\n\
         Files { Files(open: () -> Handle, same: (a: Handle, b: Handle) -> Bool) }\n\
         @main\nmain = (f ! Files) { f.open() == f.open() }\n",
    );
    let wasm = wasi(dir.path(), Level::O0);
    assert!(imports(&wasm).contains(&("ilang:host/v1".into(), "reference_id".into())));
    // Without host references, `reference_id` is defined in the module.
    let dir = package("@main\nmain = () { Nil }\n");
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
    let twice = "@host(\"i:io/v1\")\nMine { Mine(print: (text: String) -> Nil) }\n\
                 @main\nmain = (io ! Mine) { io.print(\"x\") }\n";
    assert!(build_error(twice).contains("E0311"));
    let other = "@host(\"i:io/v1\")\nInput { Input(read: () -> String) }\n\
                 @main\nmain = (input ! Input) { input.read() }\n";
    let dir = package(other);
    let wasm = wasi(dir.path(), Level::O0);
    assert!(imports(&wasm).contains(&("i:io/v1".into(), "read".into())));
}

/// The runtime imports `fd_write` and `proc_exit` itself (B.7), so a
/// program may not import them, as an external function or as a field of
/// a host service. Other WASI functions are imports like any other.
#[test]
fn the_wasi_imports_of_the_runtime_are_refused() {
    let wasi_module = "wasi_snapshot_preview1";
    for name in ["fd_write", "proc_exit"] {
        let external = format!(
            "@external(\"{wasi_module}\", \"{name}\")\nf: (code: String) -> Nil\n\
             @main\nmain = () {{ f(\"x\") }}\n"
        );
        let e = build_error(&external);
        assert!(e.contains("the runtime imports"), "{name}: {e}");
        let service = format!(
            "@host(\"{wasi_module}\")\nW {{ W({name}: (code: Bool) -> Nil) }}\n\
             @main\nmain = (w ! W) {{ w.{name}(True) }}\n"
        );
        let e = build_error(&service);
        assert!(e.contains("the runtime imports"), "{name}: {e}");
    }
    let other = format!(
        "@external(\"{wasi_module}\", \"sched_yield\")\nyield: () -> Int\n\
         @main\nmain = () {{ yield() }}\n"
    );
    let dir = package(&other);
    let wasm = wasi(dir.path(), Level::O0);
    assert!(imports(&wasm).contains(&(wasi_module.into(), "sched_yield".into())));
    // On the target of the binary interface they are normal imports.
    let dir = package(&format!(
        "@external(\"{wasi_module}\", \"proc_exit\")\nexit: (code: Int) -> Nil\n\
         @main\nmain = () {{ exit(0) }}\n"
    ));
    assert!(build(dir.path(), options(Level::O0, Target::Ilang)).is_ok());
}

#[test]
fn only_wasi_modules_name_their_target() {
    let dir = package("@main\nmain = () { Nil }\n");
    let ilang = build(dir.path(), options(Level::O0, Target::Ilang))
        .unwrap()
        .wasm;
    assert_eq!(Abi::from_wasm(&ilang).unwrap().unwrap().target, None);
    let wasm = wasi(dir.path(), Level::O0);
    assert_eq!(
        Abi::from_wasm(&wasm).unwrap().unwrap().target.as_deref(),
        Some("wasi")
    );
}
