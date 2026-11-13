//! Native packages (docs/design/native.md, N1, N2, N4 and §11): `i build`
//! makes one executable that runs without `i`.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn i(args: &[&str], project: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap()
}

/// Build the package in `project` into `dir/name`, and give its path.
fn build(project: &Path, dir: &Path, name: &str) -> PathBuf {
    let exe = dir.join(name);
    let out = i(&["build", "--out", exe.to_str().unwrap()], project);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    exe
}

/// Run an executable with an empty environment, as on a machine without `i`.
fn run(exe: &Path, args: &[&str]) -> Output {
    retry_executable(|| Command::new(exe).args(args).env_clear().output()).unwrap()
}

fn retry_executable<T>(mut run: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    for _ in 0..20 {
        match run() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(50));
            }
            result => return result,
        }
    }
    run()
}

fn package(dir: &Path, main: &str) -> PathBuf {
    let project = dir.join("app");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(project.join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::write(project.join("src/main.i"), main).unwrap();
    project
}

#[test]
fn hello_is_a_native_executable() {
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&example("hello"), dir.path(), "hello");
    let bytes = std::fs::read(&exe).unwrap();
    assert!(bytes.starts_with(b"\x7fELF") || bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]));
    let out = run(&exe, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"Hello from i!\n");
    assert!(out.stderr.is_empty());
    // The executable is the runner and the payload, and the runner is the
    // one `i` carries.
    assert!(bytes.starts_with(ilang_cli::RUNNER));
    let (start, payload) = ilang_host::native::Payload::decode(&bytes).unwrap();
    assert_eq!(start, ilang_cli::RUNNER.len());
    assert_eq!(payload.manifest.name, "hello");
}

#[test]
fn default_output_and_named_entries() {
    let dir = tempfile::tempdir().unwrap();
    let project = package(
        dir.path(),
        "(i/stdio)\n\nmain = () { stdio.print(\"main\") }\n\nother = () { stdio.print(\"other\") }\n",
    );
    let manifest = project.join("i.toml");
    std::fs::write(
        &manifest,
        "name = \"app\"\n\n[entries]\nother = \"main.other\"\n",
    )
    .unwrap();
    let out = i(&["build"], &project);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(run(&project.join("build/app"), &[]).stdout, b"main\n");
    let out = i(&["build", "other"], &project);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(run(&project.join("build/other"), &[]).stdout, b"other\n");
    // The host target still writes the Wasm module.
    let wasm = dir.path().join("hello.wasm");
    let out = i(
        &["build", "--target", "host", "--out", wasm.to_str().unwrap()],
        &example("hello"),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(std::fs::read(&wasm).unwrap().starts_with(b"\0asm"));
}

#[test]
fn arguments_and_exit_status() {
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&example("cli-log"), dir.path(), "cli_log");
    let out = run(&exe, &["--name", "world", "--count", "2"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"hello world\nhello world\ncli ok\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("info \"greeting\""));
    let out = run(&exe, &["--nope"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("Usage:"));
}

#[test]
fn stop_environment_and_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let project = package(
        dir.path(),
        r#"(
	i/process
	i/stdio
)

main = () {
	stdio.print(process.env("GREETING"))
	panic("stopped")
}
"#,
    );
    let exe = build(&project, dir.path(), "app_exe");
    let out = retry_executable(|| {
        Command::new(&exe)
            .env_clear()
            .env("GREETING", "hi")
            .output()
    })
    .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"hi\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("stopped"));
}

#[test]
fn http_get_over_tls() {
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&example("http-get"), dir.path(), "http_get");
    let out = run(&exe, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.ends_with(b"http ok\n"));
}

struct Server(std::process::Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn echo_server() {
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&example("echo"), dir.path(), "echo");
    let child = retry_executable(|| {
        Command::new(&exe)
            .env_clear()
            .stdout(Stdio::piped())
            .spawn()
    })
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
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
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

fn rejected(exe: &Path) -> String {
    let out = run(exe, &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    String::from_utf8(out.stderr).unwrap()
}

fn write_executable(path: &Path, bytes: &[u8]) {
    {
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn damaged_payloads_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&example("hello"), dir.path(), "hello");
    let bytes = std::fs::read(&exe).unwrap();
    let damaged = dir.path().join("damaged");
    for cut in [1, 100, 10_000] {
        write_executable(&damaged, &bytes[..bytes.len() - cut]);
        let error = rejected(&damaged);
        assert!(
            error.contains("payload of this executable is missing, truncated or damaged"),
            "{error}"
        );
    }
    // One changed byte in the code, the manifest or the trailer.
    let (start, _) = ilang_host::native::Payload::decode(&bytes).unwrap();
    for at in [start + 2, bytes.len() - 200, bytes.len() - 20] {
        let mut changed = bytes.clone();
        changed[at] ^= 0x40;
        write_executable(&damaged, &changed);
        let error = rejected(&damaged);
        assert!(error.contains("missing, truncated or damaged"), "{error}");
    }
    // The bare runner has no program.
    write_executable(&damaged, ilang_cli::RUNNER);
    assert!(rejected(&damaged).contains("missing, truncated or damaged"));
}

#[test]
fn another_contract_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&example("hello"), dir.path(), "hello");
    let bytes = std::fs::read(&exe).unwrap();
    let (start, payload) = ilang_host::native::Payload::decode(&bytes).unwrap();
    let rewrite = |change: &dyn Fn(&mut ilang_host::native::Payload)| {
        let mut payload = payload.clone();
        change(&mut payload);
        let mut file = bytes[..start].to_vec();
        file.extend(payload.encode().unwrap());
        let path = dir.path().join("rewritten");
        write_executable(&path, &file);
        rejected(&path)
    };
    // Another Wasmtime version, consistently re-sealed.
    let error = rewrite(&|p| {
        p.manifest.contract = p
            .manifest
            .contract
            .replace("wasmtime 50.0.0", "wasmtime 49.0.0")
    });
    assert!(error.contains("compiled for another runner"), "{error}");
    // Another target triple.
    let error = rewrite(&|p| {
        p.manifest.contract = p
            .manifest
            .contract
            .replace(ilang_cli::RUNNER_TARGET, "riscv64gc-unknown-linux-gnu")
    });
    assert!(error.contains("compiled for another runner"), "{error}");
    // A collector the code was not compiled for: the contract names it, so
    // a manifest that changes both is caught by Wasmtime's own check.
    let error = rewrite(&|p| {
        p.manifest.contract = p
            .manifest
            .contract
            .replace("collector copying", "collector drc");
        p.manifest.collector = "drc".into();
    });
    assert!(error.contains("loading the compiled program"), "{error}");
    // Another payload format.
    let error = rewrite(&|p| p.manifest.format = 2);
    assert!(error.contains("format 2"), "{error}");
    // The binding plan must be the one of the compiled module.
    let error = rewrite(&|p| p.manifest.binding_plan.artifact_digest = "sha256-0".into());
    assert!(error.contains("artifact digest"), "{error}");
}

#[test]
fn build_and_runner_share_one_contract() {
    // The runner and `i build` both use `ilang_host::native`; the payload
    // records the contract of the compiling side, and the runner accepted
    // it in the tests above. Here: it is the contract of the runner's
    // target (not the target of `i`), and names the pinned Wasmtime of
    // crates/ilang-host/Cargo.toml.
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&example("hello"), dir.path(), "hello");
    let (_, payload) = ilang_host::native::Payload::decode(&std::fs::read(&exe).unwrap()).unwrap();
    assert_eq!(
        payload.manifest.contract,
        ilang_host::native::contract(
            ilang_cli::RUNNER_TARGET,
            wasmtime::Collector::Copying,
            256 << 20
        )
    );
    let manifest = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../ilang-host/Cargo.toml"),
    )
    .unwrap();
    assert!(manifest.contains(&format!(
        "wasmtime = {{ version = \"={}\"",
        ilang_host::native::WASMTIME
    )));
    assert!(
        payload
            .manifest
            .contract
            .contains(&format!("target {}", ilang_cli::RUNNER_TARGET))
    );
    if cfg!(target_os = "linux") {
        assert!(ilang_cli::RUNNER_TARGET.ends_with("-unknown-linux-musl"));
        // `i` itself is not the runner's target, so it refuses to load the
        // payload: the code is for the runner.
        if ilang_host::native::triple() != ilang_cli::RUNNER_TARGET {
            let error = ilang_host::native::load(&std::fs::read(&exe).unwrap())
                .err()
                .unwrap()
                .to_string();
            assert!(error.contains("compiled for another runner"), "{error}");
        }
    }
}

/// The program headers of an ELF64 little-endian file: (type, offset, size).
fn program_headers(bytes: &[u8]) -> Vec<(u32, usize, usize)> {
    assert_eq!(&bytes[..4], b"\x7fELF");
    assert_eq!(bytes[4], 2, "ELF64");
    assert_eq!(bytes[5], 1, "little-endian");
    let u16_at = |at: usize| u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()) as usize;
    let u64_at = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
    let (offset, size, count) = (u64_at(0x20), u16_at(0x36), u16_at(0x38));
    (0..count)
        .map(|i| {
            let at = offset + i * size;
            let kind = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            (kind, u64_at(at + 8), u64_at(at + 32))
        })
        .collect()
}

#[cfg(target_os = "linux")]
#[test]
fn packages_are_statically_linked() {
    const PT_DYNAMIC: u32 = 2;
    const PT_INTERP: u32 = 3;
    const DT_NEEDED: u64 = 1;
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&example("hello"), dir.path(), "hello");
    let bytes = std::fs::read(&exe).unwrap();
    let headers = program_headers(&bytes);
    assert!(
        !headers.iter().any(|(kind, _, _)| *kind == PT_INTERP),
        "the package asks for a dynamic loader"
    );
    // A static PIE has a dynamic section for its own relocations, but
    // needs no shared library.
    for (_, offset, size) in headers.iter().filter(|(kind, _, _)| *kind == PT_DYNAMIC) {
        for entry in bytes[*offset..offset + size].as_chunks::<16>().0 {
            let tag = u64::from_le_bytes(entry[..8].try_into().unwrap());
            assert_ne!(tag, DT_NEEDED, "the package needs a shared library");
        }
    }
    // It runs alone in an empty directory, with no environment at all (no
    // PATH, no LD_LIBRARY_PATH) and no access to `i` or the build tree.
    let alone = tempfile::tempdir().unwrap();
    let copy = alone.path().join("hello");
    std::fs::copy(&exe, &copy).unwrap();
    let out = retry_executable(|| {
        Command::new(&copy)
            .current_dir(alone.path())
            .env_clear()
            .output()
    })
    .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"Hello from i!\n");
}

#[test]
fn the_runner_has_no_compiler() {
    let out = Command::new(env!("CARGO"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args([
            "tree",
            "--locked",
            "--offline",
            "-p",
            "ilang-runner",
            "-e",
            "normal",
            "--prefix",
            "none",
            "-f",
            "{p} {f}",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8(out.stdout).unwrap();
    assert!(tree.contains("wasmtime v50.0.0"), "{tree}");
    // The runtime shares Cranelift's data-structure crates (entity, bforest,
    // bitset); the code generators must be absent.
    for compiler in [
        "cranelift-codegen ",
        "cranelift-frontend ",
        "cranelift-native ",
        "wasmtime-internal-cranelift ",
        "wasmtime-internal-winch ",
        "winch-codegen ",
    ] {
        assert!(
            !tree.lines().any(|line| line.starts_with(compiler)),
            "the runner depends on {compiler}:\n{tree}"
        );
    }
}

#[test]
fn imports_the_runner_cannot_supply_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let project = package(
        dir.path(),
        "@external(\"plugin:raw/v1\", \"number\")\nnumber: (Float) -> Float\n\nmain = () {\n  number(1.0)\n  Nil\n}\n",
    );
    let out = i(&["build"], &project);
    assert!(!out.status.success());
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(
        error.contains("cannot supply this required host import: plugin:raw/v1 number")
            && error.contains("--target host"),
        "{error}"
    );
    // The module itself still builds for an embedding host.
    assert!(i(&["build", "--target", "host"], &project).status.success());
    // A service replaced with an ordinary i value needs no import (N4).
    let project = package(
        dir.path(),
        r#"(
	(Io) = i/io
)

greet = () <- (io: Io) { io.print("quiet") }

main = () { greet() <- (Io(print = (text) { Nil })) }
"#,
    );
    let exe = build(&project, dir.path(), "quiet");
    let out = run(&exe, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
}

#[test]
fn packages_need_the_program_entry() {
    let dir = tempfile::tempdir().unwrap();
    let project = package(
        dir.path(),
        " (i/core)\n\n (double)\n\ndouble = (n: Int) { core.inspect(n * 2) }\n",
    );
    std::fs::write(
        project.join("i.toml"),
        "name = \"app\"\n\n[wasm]\nexports = [\"app/main.double\"]\n",
    )
    .unwrap();
    let out = i(&["build"], &project);
    assert!(!out.status.success());
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(
        error.contains("program entry") && error.contains("--target host"),
        "{error}"
    );
}

#[test]
fn start_up_is_quick() {
    let dir = tempfile::tempdir().unwrap();
    let built = Instant::now();
    let exe = build(&example("hello"), dir.path(), "hello");
    let build_time = built.elapsed();
    run(&exe, &[]);
    let start = Instant::now();
    for _ in 0..5 {
        assert!(run(&exe, &[]).status.success());
    }
    let each = start.elapsed() / 5;
    // About: no compilation happens at start-up. Relative, not a wall-clock bound, so a busy
    // machine does not fail it: running the executable must cost less than building it (a
    // build compiles the program; start-up only loads the precompiled payload), and both are
    // slowed alike by load.
    assert!(
        each < build_time,
        "a run took {each:?}, the build took {build_time:?}"
    );
}
