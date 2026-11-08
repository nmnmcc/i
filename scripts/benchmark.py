#!/usr/bin/env python3
"""Measure iLang development and language compilation in disposable workspaces."""

from __future__ import annotations

import argparse
from collections import defaultdict
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import resource
import shutil
import signal
import statistics
import subprocess
import sys
import tempfile
import threading
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
IGNORED = {".git", ".devenv", ".direnv", "target", "build", "node_modules", "playwright-report", "test-results", ".ilang", ".benchmark-blaze", "__pycache__"}


class BenchmarkFailure(RuntimeError):
    pass


def resolve_executable(value: str) -> str:
    candidate = shutil.which(value)
    if candidate is None:
        raise BenchmarkFailure(f"executable not found: {value}")
    # Preserve a rustup proxy's argv[0]; resolving the cargo symlink would invoke
    # rustup itself instead of its cargo dispatch path.
    return str(Path(candidate).absolute())


def run_info(argv: list[str], env: dict[str, str]) -> dict:
    try:
        result = subprocess.run(argv, env=env, capture_output=True, text=True, timeout=120)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise BenchmarkFailure(f"cannot inspect {' '.join(argv)}: {error}") from error
    if result.returncode:
        raise BenchmarkFailure(f"{' '.join(argv)} exited {result.returncode}: {result.stderr.strip()}")
    return {"argv": argv, "stdout": result.stdout.strip(), "stderr": result.stderr.strip()}


def ignored_names(_directory: str, names: list[str]) -> set[str]:
    ignored = {name for name in names if name in IGNORED or name == ".env" or name.startswith(".env.")}
    # `build` is also a real Rust source module in the imported frontend. Only
    # discard package build outputs, not every directory with that name.
    directory = Path(_directory)
    if "build" in ignored and not any((directory / manifest).is_file() for manifest in ("Cargo.toml", "i.toml")):
        ignored.remove("build")
    return ignored


def tree_digest(root: Path) -> str:
    digest = hashlib.sha256()
    for directory, subdirs, files in os.walk(root, followlinks=False):
        subdirs[:] = sorted(name for name in subdirs if name not in ignored_names(directory, subdirs))
        for name in sorted(files):
            if name in ignored_names(directory, files):
                continue
            path = Path(directory) / name
            digest.update(str(path.relative_to(root)).encode())
            digest.update(b"\0")
            if path.is_symlink():
                digest.update(b"link\0" + os.readlink(path).encode())
            else:
                with path.open("rb") as source:
                    for block in iter(lambda: source.read(1024 * 1024), b""):
                        digest.update(block)
            digest.update(b"\0")
    return digest.hexdigest()


def snapshot(source: Path, destination: Path) -> None:
    shutil.copytree(source, destination, ignore=ignored_names, symlinks=True)


def file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def launcher_rss_floor() -> tuple[int, str]:
    # Linux RUSAGE_SELF can also include a pre-exec ancestor's high-water mark;
    # VmHWM describes this interpreter's current address space instead.
    if sys.platform.startswith("linux"):
        try:
            for line in Path("/proc/self/status").read_text().splitlines():
                if line.startswith("VmHWM:"):
                    return int(line.split()[1]) * 1024, "linux-vmhwm"
        except (OSError, ValueError, IndexError):
            pass
    scale = 1 if sys.platform == "darwin" else 1024
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * scale, "rusage-self"


def host_info() -> dict:
    info = {"system": platform.system(), "release": platform.release(), "machine": platform.machine(), "cpu_count": os.cpu_count(), "cpu_model": platform.processor() or None}
    if hasattr(os, "getloadavg"):
        info["load_average_at_start"] = list(os.getloadavg())
    if sys.platform.startswith("linux"):
        try:
            for line in Path("/proc/cpuinfo").read_text().splitlines():
                if line.startswith("model name"):
                    info["cpu_model"] = line.split(":", 1)[1].strip()
                    break
            for line in Path("/proc/meminfo").read_text().splitlines():
                if line.startswith("MemTotal:"):
                    info["memory_bytes"] = int(line.split()[1]) * 1024
                    break
        except (OSError, ValueError, IndexError):
            pass
    return info


def add_rust_probe(workspace: Path) -> tuple[Path, Path]:
    main = workspace / "crates/ilang-cli/src/main.rs"
    if not main.is_file() or main.is_symlink() or not main.resolve().is_relative_to(workspace.resolve()):
        raise BenchmarkFailure(f"expected a regular CLI entry point inside the snapshot: {main}")
    text = main.read_text()
    match = re.search(r"(?m)^(?:pub\s+)?fn\s+main\s*\(", text)
    if match is None:
        raise BenchmarkFailure("cannot locate the CLI main function for the isolated edit probe")
    opening = text.find("{", match.end())
    if opening < 0 or opening - match.end() > 1000:
        raise BenchmarkFailure("cannot safely instrument the CLI main function")
    probe = main.parent / "ilang_benchmark_probe.rs"
    types = main.parent / "ilang_benchmark_probe_types.rs"
    if probe.exists() or types.exists():
        raise BenchmarkFailure("benchmark probe names already occur in the source; refusing to overwrite them")
    main.write_text(text[: opening + 1] + "\n    ilang_benchmark_probe::verify();\n" + text[opening + 1 :] + "\nmod ilang_benchmark_probe;\nmod ilang_benchmark_probe_types;\n")
    write_rust_probe(probe, 0)
    write_rust_types(types, 0)
    return probe, types


def write_rust_probe(path: Path, revision: int) -> None:
    path.write_text(
        "#[inline(never)]\npub fn touch() -> u64 {\n"
        f"    let probe = std::hint::black_box(super::ilang_benchmark_probe_types::Probe::new({revision}));\n"
        "    std::hint::black_box(probe.value)\n}\n"
        "\npub fn verify() {\n"
        "    let actual = touch();\n"
        "    if let Ok(expected) = std::env::var(\"ILANG_BENCHMARK_EXPECTED_REVISION\") {\n"
        "        assert_eq!(actual, expected.parse::<u64>().expect(\"benchmark revision\"), \"stale function body\");\n"
        "    }\n"
        "    if let Ok(expected) = std::env::var(\"ILANG_BENCHMARK_EXPECTED_LAYOUT\") {\n"
        "        assert_eq!(super::ilang_benchmark_probe_types::layout_size(), expected.parse::<usize>().expect(\"benchmark layout\"), \"stale type layout\");\n"
        "    }\n}\n"
        "\n#[cfg(test)]\n#[test]\nfn benchmark_probe_runs() { verify(); }\n"
    )


def write_rust_types(path: Path, revision: int) -> None:
    path.write_text(
        "#[allow(dead_code)]\npub struct Probe {\n    pub value: u64,\n"
        f"    padding: [u64; {revision}],\n}}\n"
        "impl Probe {\n    #[inline(never)]\n    pub fn new(value: u64) -> Self {\n"
        f"        Self {{ value, padding: [0; {revision}] }}\n    }}\n}}\n"
        "\n#[inline(never)]\npub fn layout_size() -> usize { std::mem::size_of::<Probe>() }\n"
    )


def make_i_fixture(project: Path, modules: int, function_revision: int = 0, type_revision: int = 0) -> None:
    (project / "src").mkdir(parents=True, exist_ok=True)
    (project / "test").mkdir(exist_ok=True)
    (project / "i.toml").write_text('name = "benchmark"\nversion = "0.1.0"\ntarget = "wasm"\n')
    for index in range(modules):
        if index + 1 < modules:
            body = f"import bench_module_{index + 1}\n\npub fn apply(input: bench_types.Input) -> Int {{\n  bench_module_{index + 1}.apply(input) + 1\n}}\n"
        else:
            body = f"\npub fn apply(input: bench_types.Input) -> Int {{\n  input.value + {function_revision}\n}}\n"
        (project / "src" / f"bench_module_{index}.i").write_text("import bench_types\n" + body)
    update_i_type(project, modules, function_revision, type_revision)


def update_i_type(project: Path, modules: int, function_revision: int, revision: int) -> None:
    fields = "value: Int" + (f", generation_{revision}: Int" if revision else "")
    args = "value: 1" + (f", generation_{revision}: {revision}" if revision else "")
    (project / "src/bench_types.i").write_text(f"pub type Input {{\n  Input({fields})\n}}\n")
    (project / "src/benchmark.i").write_text(
        "import bench_module_0\nimport bench_types\n\npub fn main() {\n"
        f"  echo bench_module_0.apply(bench_types.Input({args}))\n"
        "  Nil\n}\n"
    )
    (project / "test/benchmark_test.i").write_text(
        "import bench_module_0\nimport bench_types\n\npub fn arithmetic_test() {\n"
        f"  let assert {modules + function_revision} = bench_module_0.apply(bench_types.Input({args}))\n"
        "  Nil\n}\n"
    )


def edit_i_function(project: Path, modules: int, revision: int) -> None:
    (project / "src" / f"bench_module_{modules - 1}.i").write_text(
        "import bench_types\n\npub fn apply(input: bench_types.Input) -> Int {\n"
        f"  input.value + {revision}\n}}\n"
    )


class MemorySampler:
    """Sample the RSS sum of processes carrying this isolated engine's marker."""

    def __init__(self, marker: str):
        self.marker = ("ILANG_BENCHMARK_SESSION=" + marker).encode()
        self.pids: set[int] = set()
        self.last_discovery = 0.0
        self.peak: int | None = None
        self.samples = 0
        self.available = sys.platform.startswith("linux") and Path("/proc").is_dir()

    def sample(self, discover: bool = False) -> None:
        if not self.available:
            return
        now = time.monotonic()
        if discover or now - self.last_discovery >= 0.25:
            for directory in Path("/proc").iterdir():
                if not directory.name.isdigit():
                    continue
                try:
                    if self.marker in (directory / "environ").read_bytes().split(b"\0"):
                        self.pids.add(int(directory.name))
                except (OSError, PermissionError):
                    pass
            self.last_discovery = now
        total = 0
        found = False
        for pid in self.pids.copy():
            try:
                pages = int(Path(f"/proc/{pid}/statm").read_text().split()[1])
            except (OSError, ValueError, IndexError):
                self.pids.discard(pid)
                continue
            total += pages * os.sysconf("SC_PAGE_SIZE")
            found = True
        if found:
            self.peak = max(self.peak or 0, total)
            self.samples += 1


class Harness:
    def __init__(self, args: argparse.Namespace, report: dict, output: Path):
        self.args = args
        self.report = report
        self.output = output
        self.logs = output.with_suffix("") / "logs"
        self.logs.mkdir(parents=True, exist_ok=True)
        self.blaze_sessions: list[tuple[str, Path, dict[str, str]]] = []

    def save(self) -> None:
        self.output.parent.mkdir(parents=True, exist_ok=True)
        groups: dict[str, list[dict]] = defaultdict(list)
        for run in self.report["runs"]:
            groups[f"{run['suite']}/{run['engine']}/{run['scenario']}"].append(run)
        self.report["summary"] = {
            key: {
                "samples": len(runs),
                "successful_samples": sum(run["returncode"] == 0 and not run["timed_out"] and run.get("validation", {}).get("success", True) for run in runs),
                "wall_seconds_median": statistics.median(run["wall_seconds"] for run in runs),
                "wall_seconds_min": min(run["wall_seconds"] for run in runs),
                "wall_seconds_max": max(run["wall_seconds"] for run in runs),
                "command_peak_process_rss_bytes_max": max(run["command_peak_process_rss_bytes"] for run in runs),
                "sampled_engine_peak_rss_bytes_max": max((run["sampled_engine_peak_rss_bytes"] for run in runs if run["sampled_engine_peak_rss_bytes"] is not None), default=None),
                "external_command_peak_rss_bytes_max": max((run["external_command_peak_rss_bytes"] for run in runs if "external_command_peak_rss_bytes" in run), default=None),
            }
            for key, runs in groups.items()
        }
        temporary = self.output.with_suffix(self.output.suffix + ".tmp")
        temporary.write_text(json.dumps(self.report, indent=2) + "\n")
        temporary.replace(self.output)

    def measure(self, suite: str, engine: str, scenario: str, iteration: int, argv: list[str], cwd: Path, env: dict[str, str]) -> None:
        label = f"{len(self.report['runs']):03d}-{suite}-{engine}-{scenario}-{iteration}"
        out_path, err_path = self.logs / f"{label}.stdout", self.logs / f"{label}.stderr"
        resource_path = self.logs / f"{label}.maxrss"
        measured_argv = ([self.args.gnu_time, "-f", "%M", "-o", str(resource_path), "--", *argv] if self.args.gnu_time else argv)
        memory = MemorySampler(env["ILANG_BENCHMARK_SESSION"])
        memory.sample(discover=True)
        done = threading.Event()
        waited: dict = {}
        rss_scale = 1 if sys.platform == "darwin" else 1024
        launcher_rss, launcher_rss_method = launcher_rss_floor()
        with out_path.open("wb") as stdout, err_path.open("wb") as stderr:
            started = time.perf_counter()
            try:
                process = subprocess.Popen(measured_argv, cwd=cwd, env=env, stdout=stdout, stderr=stderr, start_new_session=True)
            except OSError as error:
                raise BenchmarkFailure(f"cannot start {argv[0]}: {error}") from error

            def wait() -> None:
                try:
                    _pid, status, usage = os.wait4(process.pid, 0)
                    waited.update(status=status, usage=usage, finished=time.perf_counter())
                except OSError as error:
                    waited["error"] = str(error)
                finally:
                    done.set()

            waiter = threading.Thread(target=wait, daemon=True)
            waiter.start()
            timed_out = False
            def terminate() -> None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                done.wait(2)
                # A group leader can exit on SIGTERM while a compiler/test
                # descendant ignores it. Kill the group even if we reaped the
                # leader; otherwise survivors contaminate later samples.
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                done.wait()

            try:
                while not done.wait(0.05):
                    memory.sample()
                    if time.perf_counter() - started > self.args.timeout:
                        timed_out = True
                        terminate()
            except BaseException:
                terminate()
                waiter.join()
                if "status" in waited:
                    process.returncode = os.waitstatus_to_exitcode(waited["status"])
                raise
            waiter.join()
            memory.sample(discover=True)
            if "status" not in waited:
                raise BenchmarkFailure("could not collect child process resource usage")
            process.returncode = os.waitstatus_to_exitcode(waited["status"])
        run = {
            "suite": suite, "engine": engine, "scenario": scenario, "iteration": iteration,
            "argv": argv, "cwd": str(cwd), "returncode": process.returncode, "timed_out": timed_out,
            "wall_seconds": waited["finished"] - started,
            "command_peak_process_rss_bytes": waited["usage"].ru_maxrss * rss_scale,
            "launcher_peak_rss_bytes_at_spawn": launcher_rss,
            "launcher_peak_rss_method": launcher_rss_method,
            "command_rss_at_or_below_launcher_floor": waited["usage"].ru_maxrss * rss_scale <= launcher_rss,
            "sampled_engine_peak_rss_bytes": memory.peak, "engine_rss_samples": memory.samples,
            "stdout": str(out_path), "stderr": str(err_path),
        }
        self.report["runs"].append(run)
        if self.args.gnu_time:
            run["resource_helper_argv"] = measured_argv
            run["resource_helper_log"] = str(resource_path)
            try:
                run["external_command_peak_rss_bytes"] = int(resource_path.read_text().strip().splitlines()[-1]) * 1024
            except (OSError, ValueError, IndexError) as error:
                run["resource_helper_error"] = str(error)
                self.save()
                raise BenchmarkFailure(f"could not read GNU Time resource data for {label}") from error
        self.save()
        print(f"{suite}/{engine}/{scenario} [{iteration}]: {run['wall_seconds']:.6f}s exit={process.returncode}", flush=True)
        if timed_out or process.returncode:
            raise BenchmarkFailure(f"{label} {'timed out' if timed_out else 'failed'}; see {err_path}")

    def rust(self, directory: Path) -> None:
        if not (self.args.source / "Cargo.lock").is_file():
            raise BenchmarkFailure("Rust benchmarks require an existing Cargo.lock; generate and validate it first")
        reference = directory / "rust-source"
        snapshot(self.args.source, reference)
        self.report["rust_source_sha256"] = tree_digest(reference)
        cargo = resolve_executable(self.args.cargo)
        for engine in self.args.engines:
            workspace = directory / f"rust-{engine}"
            snapshot(reference, workspace)
            probe, types = add_rust_probe(workspace)
            env = os.environ.copy()
            env.update(ILANG_ROOT=str(workspace), ILANG_BENCHMARK_SESSION=str(uuid.uuid4()), ILANG_BENCHMARK_EXPECTED_REVISION="0", ILANG_BENCHMARK_EXPECTED_LAYOUT="8")
            if (workspace / "stdlib/i.toml").is_file():
                env["ILANG_STDLIB"] = str(workspace / "stdlib")
            else:
                env.pop("ILANG_STDLIB", None)
            for variable in ("CARGO", "RUSTC", "RUSTDOC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTUP_TOOLCHAIN"):
                env.pop(variable, None)
            if engine == "cargo":
                executable = cargo
                env["PATH"] = str(Path(cargo).parent) + os.pathsep + env.get("PATH", "")
                env["CARGO_TARGET_DIR"] = str(workspace / "target/cargo")
                binary = workspace / "target/cargo/debug/i"
                build = [executable, "build", "--locked", "--bin", "i"]
                tests = [executable, "test", "--locked"]
            else:
                executable = resolve_executable(self.args.blaze)
                env.pop("CARGO_TARGET_DIR", None)
                env["BLAZE_HOME"] = str(workspace / ".benchmark-blaze")
                env["RA_JIT_CACHE"] = str(workspace / ".benchmark-blaze/cache")
                binary = workspace / "target/debug/i"
                env.pop("BLAZE_TESTS", None)
                env.pop("RA_JIT_TESTS", None)
                self.blaze_sessions.append((executable, workspace, env))
                build = [executable, "build", "--bin", "i"]
                tests = [executable, "test"]
            self.report["tools"][engine] = run_info([executable, "--version"], env)
            if engine == "blaze":
                self.report["tools"]["blaze_toolchain"] = run_info([executable, "toolchain"], env)
            else:
                self.report["tools"]["rustc"] = run_info([str(Path(cargo).parent / "rustc"), "--version"], env)
            self.measure("rust", engine, "cold-build", 0, build, workspace, env)
            self.validate_rust_binary(binary, workspace, env)
            for iteration in range(self.args.rounds):
                self.measure("rust", engine, "no-change-build", iteration, build, workspace, env)
                self.validate_rust_binary(binary, workspace, env)
            for iteration in range(self.args.rounds):
                write_rust_probe(probe, iteration + 1)
                env["ILANG_BENCHMARK_EXPECTED_REVISION"] = str(iteration + 1)
                self.measure("rust", engine, "function-edit-build", iteration, build, workspace, env)
                self.validate_rust_binary(binary, workspace, env)
            for iteration in range(self.args.rounds):
                write_rust_types(types, iteration + 1)
                env["ILANG_BENCHMARK_EXPECTED_LAYOUT"] = str((iteration + 2) * 8)
                self.measure("rust", engine, "type-edit-build", iteration, build, workspace, env)
                self.validate_rust_binary(binary, workspace, env)
            self.measure("rust", engine, "first-test", 0, tests, workspace, env)
            self.validate_rust_tests(engine)
            for iteration in range(self.args.rounds):
                write_rust_probe(probe, self.args.rounds + iteration + 1)
                env["ILANG_BENCHMARK_EXPECTED_REVISION"] = str(self.args.rounds + iteration + 1)
                self.measure("rust", engine, "function-edit-test", iteration, tests, workspace, env)
                self.validate_rust_tests(engine)

    def validate_rust_tests(self, engine: str) -> None:
        run = self.report["runs"][-1]
        output = "\n".join(Path(run[stream]).read_text(errors="replace") for stream in ("stdout", "stderr"))
        output = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", output)
        summaries = re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;", output)
        fields = ("passed", "failed", "ignored", "measured", "filtered")
        totals = {field: sum(int(summary[index]) for summary in summaries) for index, field in enumerate(fields)}
        probe_ran = bool(re.search(r"(?m)^test ilang_benchmark_probe::benchmark_probe_runs \.\.\. ok\s*$", output))
        expected = self.report.setdefault("rust_test_expected_passed", totals["passed"])
        self.report.setdefault("rust_test_oracle_engine", engine)
        validation = {
            "success": bool(summaries) and totals["passed"] > 0 and all(totals[field] == 0 for field in fields[1:]) and probe_ran and totals["passed"] == expected,
            "summary_count": len(summaries), "tests": totals, "probe_executed": probe_ran,
            "expected_passed": expected, "oracle_engine": self.report["rust_test_oracle_engine"],
        }
        run["validation"] = validation
        self.save()
        if not validation["success"]:
            raise BenchmarkFailure(f"{engine} did not execute the complete expected Rust test suite: {validation}")

    def validate_rust_binary(self, binary: Path, workspace: Path, env: dict[str, str]) -> None:
        # A build that ignores an edit must fail the benchmark, not appear fast.
        argv = [str(binary), "--version"]
        try:
            result = subprocess.run(argv, cwd=workspace, env=env, capture_output=True, text=True, timeout=30)
        except (OSError, subprocess.TimeoutExpired) as error:
            validation = {"success": False, "argv": argv, "error": str(error)}
        else:
            validation = {"success": result.returncode == 0, "argv": argv, "returncode": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
        validation.update(expected_revision=env["ILANG_BENCHMARK_EXPECTED_REVISION"], expected_layout_bytes=env["ILANG_BENCHMARK_EXPECTED_LAYOUT"])
        self.report["runs"][-1]["validation"] = validation
        self.save()
        if not validation["success"]:
            raise BenchmarkFailure(f"rebuilt {binary} failed its runtime revision/layout assertion: {validation}")

    def language(self, directory: Path) -> None:
        original_compiler = resolve_executable(self.args.compiler)
        original_runner = resolve_executable(self.args.runner)
        binaries = directory / "bin"
        binaries.mkdir()
        compiler, runner = str(binaries / "i"), str(binaries / "ilang-runner")
        shutil.copy2(original_compiler, compiler)
        shutil.copy2(original_runner, runner)
        stdlib_source = (self.args.stdlib or Path(os.environ.get("ILANG_STDLIB", str(self.args.source / "stdlib")))).resolve()
        if not (stdlib_source / "i.toml").is_file():
            raise BenchmarkFailure(f"standard library not found at {stdlib_source}; use --stdlib to select the matching sources")
        stdlib = directory / "stdlib"
        snapshot(stdlib_source, stdlib)
        env = os.environ.copy()
        env.update(ILANG_BENCHMARK_SESSION=str(uuid.uuid4()), ILANG_RUNNER=runner, ILANG_STDLIB=str(stdlib))
        self.report["tools"]["i"] = run_info([compiler, "--version"], env)
        self.report["input_compiler"] = original_compiler
        self.report["input_runner"] = original_runner
        self.report["input_stdlib"] = str(stdlib_source)
        self.report["compiler_sha256"] = file_digest(Path(compiler))
        self.report["runner_sha256"] = file_digest(Path(runner))
        self.report["stdlib_sha256"] = tree_digest(stdlib)
        for command in ("check", "build"):
            project = directory / f"i-{command}"
            make_i_fixture(project, self.args.modules)
            argv = [compiler, "--project", str(project), command]
            self.measure("i", "i", f"cold-{command}", 0, argv, project, env)
            if command == "build":
                self.validate_i_binary(runner, project, env, self.args.modules)
            for iteration in range(self.args.rounds):
                self.measure("i", "i", f"no-change-{command}", iteration, argv, project, env)
                if command == "build":
                    self.validate_i_binary(runner, project, env, self.args.modules)
            for iteration in range(self.args.rounds):
                edit_i_function(project, self.args.modules, iteration + 1)
                self.measure("i", "i", f"function-edit-{command}", iteration, argv, project, env)
                if command == "build":
                    self.validate_i_binary(runner, project, env, self.args.modules + iteration + 1)
            for iteration in range(self.args.rounds):
                update_i_type(project, self.args.modules, self.args.rounds, iteration + 1)
                self.measure("i", "i", f"type-edit-{command}", iteration, argv, project, env)
                if command == "build":
                    self.validate_i_binary(runner, project, env, self.args.modules + self.args.rounds)
        tests = [compiler, "--project", str(project), "test", "--json"]
        self.measure("i", "i", "first-test", 0, tests, project, env)
        self.validate_i_tests()
        for iteration in range(self.args.rounds):
            revision = self.args.rounds + iteration + 1
            edit_i_function(project, self.args.modules, revision)
            update_i_type(project, self.args.modules, revision, self.args.rounds)
            self.measure("i", "i", "function-edit-test", iteration, tests, project, env)
            self.validate_i_tests()

    def validate_i_tests(self) -> None:
        run = self.report["runs"][-1]
        validation = {"success": False}
        try:
            execution = json.loads(Path(run["stdout"]).read_text())
            results = execution["results"]
            validation.update(passed=execution["passed"], failed=execution["failed"], executed=len(results))
            validation["success"] = execution["passed"] == 1 and execution["failed"] == 0 and len(results) == 1 and results[0]["status"] == "passed" and "arithmetic_test" in results[0]["name"]
        except (OSError, ValueError, KeyError, IndexError, TypeError) as error:
            validation["error"] = str(error)
        run["validation"] = validation
        self.save()
        if not validation["success"]:
            raise BenchmarkFailure(f"Gleam runner did not execute its expected arithmetic test: {validation}")

    def validate_i_binary(self, runner: str, project: Path, env: dict[str, str], expected: int) -> None:
        # Run the artifact directly, without another compiler invocation that
        # could hide an incorrect cached build. Echo makes the leaf revision
        # observable without changing its caller during function-edit samples.
        artifact = project / "build/wasm/benchmark.wasm"
        argv = [runner, str(artifact), "--json"]
        validation = {"success": False, "argv": argv, "expected_result": expected}
        try:
            result = subprocess.run(argv, cwd=project, env=env, capture_output=True, text=True, timeout=30)
            validation.update(returncode=result.returncode, stdout=result.stdout, stderr=result.stderr, artifact_sha256=file_digest(artifact))
            execution = json.loads(result.stdout)
            actual = execution["results"][0]["stderr"].strip().splitlines()[-1]
            validation.update(actual_result=actual, success=result.returncode == 0 and execution["passed"] == 1 and execution["failed"] == 0 and actual == str(expected))
        except (OSError, subprocess.TimeoutExpired, ValueError, KeyError, IndexError) as error:
            validation["error"] = str(error)
        self.report["runs"][-1]["validation"] = validation
        self.save()
        if not validation["success"]:
            raise BenchmarkFailure(f"rebuilt Gleam artifact failed its runtime result assertion: {validation}")

    def cleanup(self) -> None:
        for executable, workspace, env in self.blaze_sessions:
            try:
                result = subprocess.run([executable, "stop"], cwd=workspace, env=env, capture_output=True, text=True, timeout=30)
                self.report.setdefault("cleanup", []).append({"engine": "blaze", "returncode": result.returncode, "stderr": result.stderr.strip()})
                if result.returncode:
                    self.report["success"] = False
            except (OSError, subprocess.TimeoutExpired) as error:
                self.report.setdefault("cleanup", []).append({"engine": "blaze", "error": str(error)})
                self.report["success"] = False


def positive(value: str) -> int:
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be positive")
    return parsed


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite", choices=("rust", "i", "all"))
    parser.add_argument("--source", type=Path, default=ROOT, help="iLang checkout to snapshot for Rust benchmarks")
    parser.add_argument("--engines", choices=("cargo", "blaze"), nargs="+", default=["cargo", "blaze"])
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--blaze", default="blaze")
    parser.add_argument("--compiler", help="prebuilt iLang binary for language compilation benchmarks")
    parser.add_argument("--runner", help="prebuilt ilang-runner binary for language test cycles")
    parser.add_argument("--stdlib", type=Path, help="matching standard library to snapshot (default: ILANG_STDLIB or SOURCE/stdlib)")
    parser.add_argument("--rounds", type=positive, default=5, help="samples for each warm/edit scenario; cold scenarios run once")
    parser.add_argument("--modules", type=positive, default=32, help="module count in the generated Gleam dependency chain")
    parser.add_argument("--timeout", type=positive, default=1800, help="per-command deadline in seconds")
    parser.add_argument("--gnu-time", help="optional GNU Time executable for separate child RSS measurement; helper startup is included in these timings")
    parser.add_argument("--output", type=Path, default=ROOT / "target/benchmarks/report.json")
    parser.add_argument("--work-parent", type=Path, help="parent directory for disposable snapshots")
    parser.add_argument("--keep-workdir", action="store_true", help="keep disposable snapshots for inspecting the measured inputs")
    args = parser.parse_args()
    args.source, args.output = args.source.resolve(), args.output.resolve()
    if args.suite in ("i", "all") and (not args.compiler or not args.runner):
        parser.error("i/all require explicit --compiler and --runner paths")
    if not hasattr(os, "wait4"):
        parser.error("this harness requires Unix wait4 resource accounting")
    if args.work_parent:
        args.work_parent = args.work_parent.resolve()
    temp_parent = args.work_parent or Path(tempfile.gettempdir()).resolve()
    if temp_parent.is_relative_to(args.source):
        parser.error("the temporary directory (--work-parent or TMPDIR) must be outside --source to prevent recursive snapshots")
    if args.suite in ("rust", "all") and "blaze" in args.engines:
        if "target" in temp_parent.parts:
            parser.error("Blaze source snapshots must not have a 'target' ancestor; this pinned version treats those sources as immutable")
    if args.work_parent:
        args.work_parent.mkdir(parents=True, exist_ok=True)
    if args.output.is_relative_to(args.source) and not any(part in IGNORED for part in args.output.relative_to(args.source).parts):
        parser.error("--output must be outside --source or within an ignored directory such as target/")
    return args


def main() -> int:
    args = arguments()
    started = datetime.now(timezone.utc).isoformat()
    work = Path(tempfile.mkdtemp(prefix="ilang-benchmark-", dir=args.work_parent))
    report = {
        "schema_version": 1, "started_utc": started, "success": False,
        "host": host_info(),
        "parameters": {key: str(value) if isinstance(value, Path) else value for key, value in vars(args).items()},
        "work_directory": str(work), "tools": {}, "runs": [],
        "measurement_notes": {
            "cold": "Fresh project target/cache and new Blaze session; system/Nix/Cargo registry caches are retained. Toolchain realization/version inspection is outside timed commands.",
            "rss": "command_peak_process_rss_bytes is wait4 ru_maxrss, a process high-water mark, not a sum. Child launch can inherit the launcher's earlier high-water mark; launcher_peak_rss_bytes_at_spawn and command_rss_at_or_below_launcher_floor identify that floor, which is not subtracted. sampled_engine_peak_rss_bytes is a Linux /proc RSS sum for the isolated engine, including inherited-marker Blaze workers; it is sampled, may miss short-lived peaks, and double-counts shared pages. Null means unavailable/not observed.",
            "observer_overhead": "Initial/final process discovery is outside the timed interval; periodic /proc discovery and sampling runs concurrently with longer commands and consumes CPU. Its effect on command wall time is not independently quantified or subtracted. Close timings should not be ranked as meaningful differences.",
            "edits": "Rust edits change generated reachable probe functions and a type shared by two CLI modules in an isolated source snapshot. Gleam edits change generated fixture modules only.",
        },
    }
    harness = Harness(args, report, args.output)
    try:
        if args.gnu_time:
            args.gnu_time = resolve_executable(args.gnu_time)
            report["tools"]["gnu_time"] = run_info([args.gnu_time, "--version"], os.environ.copy())
            if "GNU Time" not in report["tools"]["gnu_time"]["stdout"]:
                raise BenchmarkFailure("--gnu-time must identify GNU Time, whose %M output is in KiB")
            report["measurement_notes"]["external_rss"] = "external_command_peak_rss_bytes comes from GNU Time's child wait4 accounting in KiB, avoiding the Python launcher's inherited RSS floor. It is a process high-water mark, not aggregate or daemon memory. Timings in this supplementary run include helper startup overhead."
        if args.suite in ("rust", "all"):
            harness.rust(work)
        if args.suite in ("i", "all"):
            harness.language(work)
        report["success"] = True
    except (BenchmarkFailure, KeyboardInterrupt, OSError) as error:
        report["error"] = str(error) or "interrupted"
        print(f"benchmark failed: {report['error']}", file=sys.stderr)
    finally:
        harness.cleanup()
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        if hasattr(os, "getloadavg"):
            report["host"]["load_average_at_finish"] = list(os.getloadavg())
        report["work_directory_kept"] = args.keep_workdir
        harness.save()
        if not args.keep_workdir:
            shutil.rmtree(work)
        print(f"Raw report: {args.output}")
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
