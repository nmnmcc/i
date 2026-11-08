#!/usr/bin/env python3
"""Run iLang's language, upstream, host and compiler-engine conformance suites.

No case is an expected failure: unsupported features, empty test reports, missing
tools, compiler crashes and timeouts fail the suite. The small negative programs
must fail at their designated stage with the expected diagnostic.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib


ROOT = Path(__file__).resolve().parent.parent
SUITE = ROOT / "tests" / "conformance"
WASM_MAGIC = b"\0asm\x01\0\0\0"


class Failure(Exception):
    """A conformance assertion failed."""


def executable(value: str) -> str:
    resolved = shutil.which(value)
    if resolved is None and Path(value).is_file():
        resolved = str(Path(value).resolve())
    if resolved is None or not os.access(resolved, os.X_OK):
        raise Failure(f"required executable is unavailable: {value}")
    return str(Path(resolved).resolve())


def file_digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def source_fingerprint() -> str:
    """Identify compiler, runtime, standard library and executable test sources."""
    digest = hashlib.sha256()
    roots = [ROOT / "crates", ROOT / "compiler", ROOT / "stdlib", ROOT / "tests"]
    paths = {ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "rust-toolchain.toml", Path(__file__).resolve()}
    for root in roots:
        for path in root.rglob("*"):
            if not path.is_file() or any(part in {"target", ".git", "__pycache__"} for part in path.relative_to(root).parts):
                continue
            # Imported Rust code contains src/build; only package-root build
            # directories are generated outputs.
            if any(parent.name == "build" and any((parent.parent / name).is_file() for name in ("Cargo.toml", "i.toml"))
                   for parent in path.parents if parent.is_relative_to(root)):
                continue
            if path.suffix in {".rs", ".i", ".wat", ".mjs", ".json", ".py"} or path.name in {"Cargo.toml", "Cargo.lock", "i.toml"}:
                paths.add(path)
    for path in sorted(paths):
        digest.update(path.relative_to(ROOT).as_posix().encode())
        digest.update(b"\0")
        digest.update(bytes.fromhex(file_digest(path)))
    return digest.hexdigest()


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")
    temporary.replace(path)


def invoke(command: list[str], record: dict, timeout: int, cwd: Path = ROOT, env: dict | None = None) -> dict:
    started = time.monotonic()
    process = subprocess.Popen(
        command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8", errors="replace", start_new_session=os.name == "posix", env=env,
    )
    try:
        stdout, stderr = process.communicate(timeout=timeout)
        event = {
            "command": command, "cwd": str(cwd), "returncode": process.returncode,
            "stdout": stdout, "stderr": stderr,
            "elapsed_seconds": time.monotonic() - started,
        }
    except subprocess.TimeoutExpired as error:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGTERM)
        else:
            process.terminate()
        try:
            stdout, stderr = process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            if os.name == "posix":
                os.killpg(process.pid, signal.SIGKILL)
            else:
                process.kill()
            stdout, stderr = process.communicate()
        event = {
            "command": command, "cwd": str(cwd), "returncode": None,
            "stdout": stdout, "stderr": stderr,
            "elapsed_seconds": time.monotonic() - started, "timeout": timeout,
        }
        record["commands"].append(event)
        raise Failure(f"command timed out after {timeout}s: {' '.join(command)}") from error
    record["commands"].append(event)
    return event


def require_success(event: dict) -> None:
    if event["returncode"] != 0:
        raise Failure(
            f"command exited {event['returncode']}: {' '.join(event['command'])}\n"
            f"{event['stderr'][-12000:]}\n{event['stdout'][-4000:]}"
        )


def require_failure(event: dict, pattern: str) -> None:
    if event["returncode"] != 1:
        raise Failure(f"expected a diagnosed failure (exit 1), got {event['returncode']}")
    if re.search(pattern, event["stderr"] + event["stdout"]) is None:
        raise Failure(f"failure did not contain expected diagnostic {pattern!r}")


def package_name(project: Path) -> str:
    with (project / "i.toml").open("rb") as stream:
        return tomllib.load(stream)["name"]


def compiler_command(compiler: str, project: Path, runner: str, *arguments: str) -> list[str]:
    command = [compiler, "--project", str(project), *arguments]
    if "run" in arguments or "test" in arguments:
        command.extend(["--runner", runner])
    return command


def artifact(project: Path, tests: bool = False) -> Path:
    suffix = "_test" if tests else ""
    return project / "build" / "wasm" / f"{package_name(project)}{suffix}.wasm"


def remove_artifact(path: Path) -> None:
    """A fresh-build check must not accept a previous compiler's output."""
    path.unlink(missing_ok=True)
    path.with_suffix(".abi.json").unlink(missing_ok=True)


def validate_artifact(path: Path, record: dict, args: argparse.Namespace) -> bytes:
    if not path.is_file():
        raise Failure(f"compiler did not create its advertised Wasm artifact: {path}")
    data = path.read_bytes()
    if not data.startswith(WASM_MAGIC):
        raise Failure(f"output is not a Core WebAssembly module: {path}")
    require_success(invoke([args.wasm_tools, "validate", str(path)], record, args.timeout))
    return data


def check_upstream_sources() -> dict:
    manifest = json.loads((SUITE / "upstream-manifest.json").read_text())
    for entry in manifest["files"]:
        path = SUITE / entry["path"]
        if not path.is_file() or file_digest(path) != entry["sha256"]:
            raise Failure(f"preserved upstream source differs from its recorded hash: {path}")
    return {"sources": manifest["sources"], "verified_files": len(manifest["files"])}


def run_case(case: dict, compiler: str, record: dict, args: argparse.Namespace) -> tuple[Path | None, bytes | None]:
    project = (SUITE / case["project"]).resolve()
    cache_flags = ("--no-cache",) if args.no_cache or args.compare_compiler else ()
    command = lambda *parts: compiler_command(compiler, project, args.runner, *cache_flags, *parts)
    if case["kind"] == "compile_error":
        result = invoke(command(case["command"]), record, args.timeout)
        require_failure(result, case["stderr_pattern"])
        return None, None
    require_success(invoke(command("check"), record, args.timeout))
    if cache_flags:
        remove_artifact(artifact(project))
    require_success(invoke(command("build"), record, args.timeout))
    path = artifact(project)
    data = validate_artifact(path, record, args)
    result = invoke(command("run"), record, args.timeout)
    if case["kind"] == "runtime_error":
        require_failure(result, case["stderr_pattern"])
    else:
        require_success(result)
        if result["stdout"] != case["stdout"]:
            raise Failure(f"stdout mismatch: expected {case['stdout']!r}, got {result['stdout']!r}")
    return path, data


@contextmanager
def corpus_project(corpus: dict):
    if "project" in corpus:
        yield (SUITE / corpus["project"]).resolve()
        return
    with tempfile.TemporaryDirectory(prefix="ilang-stdlib-tests-") as temporary:
        project = Path(temporary)
        (project / "src").mkdir()
        (project / "test").mkdir()
        name = corpus["name"]
        (project / "i.toml").write_text(f'name = "{name}"\nversion = "0.1.0"\ntarget = "wasm"\n')
        (project / "src" / f"{name}.i").write_text("pub fn main() { Nil }\n")
        sources = sorted((SUITE / corpus["fixture_directory"]).glob("*.i"))
        if not sources:
            raise Failure(f"no source files for corpus {name}")
        for source in sources:
            shutil.copy2(source, project / "test" / source.name)
        yield project


def corpus_test_names(corpus: dict, project: Path) -> set[str]:
    if "test_names" in corpus:
        return set(corpus["test_names"])
    if corpus["name"] == "upstream_language":
        return set(json.loads((project / "migration.json").read_text())["expected_tests"])
    if corpus["name"] == "stdlib":
        migration = json.loads((project / "test" / "migration.json").read_text())
        return {f"{item['module']}.{item['function']}" for item in migration["active_tests"]}
    return {
        f"{path.relative_to(project / 'test').with_suffix('').as_posix()}.{name}"
        for path in (project / "test").rglob("*.i")
        for name in re.findall(r"^pub fn (\w+_test)\s*\(\s*\)", path.read_text(), re.M)
    }


def run_corpus(corpus: dict, record: dict, args: argparse.Namespace) -> dict:
    with corpus_project(corpus) as project:
        expected = corpus["expected_tests"]
        expected_names = corpus_test_names(corpus, project)
        if len(expected_names) != expected:
            raise Failure(f"corpus source inventory has {len(expected_names)} tests; expected {expected}")

        def execute(compiler: str) -> tuple[dict, bytes]:
            if args.no_cache or args.compare_compiler:
                remove_artifact(artifact(project, tests=True))
            result = invoke(
                compiler_command(compiler, project, args.runner,
                                 *(("--no-cache",) if args.no_cache or args.compare_compiler else ()), "--json", "test"),
                record, args.corpus_timeout,
            )
            try:
                report = json.loads(result["stdout"])
            except json.JSONDecodeError as error:
                require_success(result)
                raise Failure("test command did not produce a valid JSON result") from error
            results = report.get("results", [])
            if not isinstance(results, list) or any(not isinstance(item, dict) for item in results):
                raise Failure("test command produced a malformed results array")
            record["executed_tests"] = len(results)
            failed = [item for item in results if item.get("status") != "passed"]
            if failed:
                record["failed_tests"] = failed
                details = "\n".join(f"{item.get('name')}: {item.get('error', item.get('status'))}" for item in failed[:25])
                raise Failure(f"{len(failed)} of {len(results)} executed tests failed:\n{details}")
            require_success(result)
            if report.get("mode") != "test" or len(results) != expected:
                raise Failure(f"expected {expected} executed tests, received {len(results)} in mode {report.get('mode')!r}")
            names = [entry.get("name") for entry in results]
            if len(set(names)) != expected or set(names) != expected_names:
                missing = sorted(expected_names - set(names))[:10]
                extra = sorted(set(names) - expected_names)[:10]
                raise Failure(f"test report differs from source inventory; missing={missing}, extra={extra}")
            if report.get("passed") != expected or report.get("failed") != 0:
                raise Failure(f"test summary disagrees with expected {expected} successes")
            if any(entry.get("status") != "passed" for entry in results):
                raise Failure("test report contains a failed or unexecuted test")
            return report, validate_artifact(artifact(project, tests=True), record, args)

        report, data = execute(args.compiler)
        if args.compare_compiler:
            comparison, other = execute(args.compare_compiler)
            if data != other:
                raise Failure("Cargo and Blaze compiler builds produced different corpus Wasm bytes")
            outputs = lambda value: [(r["name"], r["stdout"], r["stderr"]) for r in value["results"]]
            if outputs(report) != outputs(comparison):
                raise Failure("Cargo and Blaze compiler builds produced different corpus behavior")
        path = artifact(project, tests=True)
        abi = json.loads(path.with_suffix(".abi.json").read_text())
        if set(abi.get("tests", [])) != expected_names:
            raise Failure("test artifact ABI does not describe exactly the executed corpus")
        destination = args.output / "modules" / f"{corpus['name']}_test.wasm"
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, destination)
        record["executed_tests"] = expected
        record["wasm_sha256"] = hashlib.sha256(data).hexdigest()
        return {
            "name": corpus["name"], "wasm": str(destination), "tests": abi["tests"],
            "stdout": "".join(result["stdout"] for result in report["results"]),
            "stderr": "".join(result["stderr"] for result in report["results"]),
        }


def migration_checks(record: dict, args: argparse.Namespace) -> None:
    sources = sorted((ROOT / "stdlib" / "test" / "migration").glob("*.i.invalid"))
    if len(sources) != 2:
        raise Failure("expected both legacy-platform migration diagnostic fixtures")
    for source in sources:
        with tempfile.TemporaryDirectory(prefix="ilang-migration-") as temporary:
            project = Path(temporary)
            (project / "src").mkdir()
            (project / "i.toml").write_text('name = "migration"\nversion = "0.1.0"\ntarget = "wasm"\n')
            shutil.copy2(source, project / "src" / "migration.i")
            result = invoke(compiler_command(args.compiler, project, args.runner, "check"), record, args.timeout)
            require_failure(result, r"(?i)wasm|target|external")


def host_abi_check(record: dict, args: argparse.Namespace) -> dict:
    project = SUITE / "projects" / "host_abi"
    expected = json.loads((project / "expected-abi.json").read_text())

    def compile_library(compiler: str) -> bytes:
        remove_artifact(artifact(project))
        require_success(invoke(
            [compiler, "--project", str(project), "--no-cache", "build", "--lib"], record, args.timeout,
        ))
        path = artifact(project)
        data = validate_artifact(path, record, args)
        if json.loads(path.with_suffix(".abi.json").read_text()) != expected:
            raise Failure("compiled host library ABI differs from the explicit contract")
        return data

    data = compile_library(args.compiler)
    if args.compare_compiler and compile_library(args.compare_compiler) != data:
        raise Failure("Cargo and Blaze produced different host ABI library Wasm bytes")
    destination = args.output / "modules" / "host_abi.wasm"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
    if args.host_probe:
        command = [executable(args.host_probe), str(destination)]
    else:
        target_dir = os.environ.get("ILANG_CARGO_TARGET_DIR", os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target" / "cargo")))
        command = [executable(args.cargo), "run", "--release", "--locked", "--target-dir", target_dir,
                   "-p", "ilang-host", "--example", "verify_compiler_abi", "--", str(destination)]
    result = invoke(command, record, args.corpus_timeout)
    require_success(result)
    try:
        host_report = json.loads(result["stdout"])
    except json.JSONDecodeError as error:
        raise Failure("native ABI probe did not produce its execution report") from error
    if host_report.get("host") != "wasmtime" or host_report.get("passed") != 44 or host_report.get("failed") != 0:
        raise Failure(f"native ABI probe did not complete all 44 assertions: {host_report}")
    record["executed_host_cases"] = 44
    record["wasm_sha256"] = hashlib.sha256(data).hexdigest()
    return {"name": "host_abi", "wasm": str(destination), "hostAbi": True, "stdout": "", "stderr": ""}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", required=True, help="iLang executable under test")
    parser.add_argument("--runner", default="ilang-runner", help="isolated Wasmtime runner")
    parser.add_argument("--wasm-tools", default="wasm-tools")
    parser.add_argument("--wasmtime", default="wasmtime")
    parser.add_argument("--cargo", default="cargo", help="pinned Cargo used to run the native ABI integration example")
    parser.add_argument("--host-probe", help="prebuilt verify_compiler_abi executable; otherwise build it with Cargo")
    parser.add_argument("--compare-compiler", help="second compiler build; require identical Wasm and behavior")
    parser.add_argument("--no-cache", action="store_true", help="force fresh frontend and code generation for debugging")
    parser.add_argument("--case", action="append", help="select named cases/corpora for focused development")
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument("--corpus-timeout", type=int, default=1800)
    parser.add_argument("--output", type=Path, default=ROOT / "target" / "conformance")
    args = parser.parse_args()
    try:
        for name in ("compiler", "runner", "wasm_tools", "wasmtime", "compare_compiler"):
            value = getattr(args, name)
            if value:
                setattr(args, name, executable(value))
        upstream = check_upstream_sources()
    except Failure as error:
        parser.error(str(error))

    manifest = json.loads((SUITE / "cases.json").read_text())
    available = {case["name"] for case in manifest["cases"] + manifest["corpora"]} | {"unicode_sources", "migration_diagnostics", "runtime_values", "host_abi"}
    selected = set(args.case or available)
    if selected - available:
        parser.error(f"unknown cases: {', '.join(sorted(selected - available))}")
    args.output = args.output.resolve()
    browser: list[dict] = []
    report = {"schema": 1, "upstream": upstream, "compiler": args.compiler,
              "started_utc": datetime.now(timezone.utc).isoformat(), "source_sha256": source_fingerprint(),
              "cache_disabled": bool(args.no_cache or args.compare_compiler),
              "compare_compiler": args.compare_compiler,
              "selected": sorted(selected), "results": []}
    tool_paths = [args.compiler, args.runner, *([args.compare_compiler] if args.compare_compiler else []),
                  *([executable(args.host_probe)] if args.host_probe else [])]
    report["tool_sha256"] = {path: file_digest(Path(path)) for path in tool_paths}
    write_json(args.output / "browser.json", browser)

    def perform(name: str, operation) -> None:
        started = time.monotonic()
        record = {"name": name, "status": "running", "commands": []}
        try:
            operation(record)
            record["status"] = "passed"
        except (Failure, OSError, ValueError, KeyError) as error:
            record["status"] = "failed"
            record["error"] = str(error)
        report["results"].append(record)
        record["elapsed_seconds"] = time.monotonic() - started
        report["passed"] = sum(item["status"] == "passed" for item in report["results"])
        report["failed"] = sum(item["status"] == "failed" for item in report["results"])
        write_json(args.output / "report.json", report)
        write_json(args.output / "browser.json", browser)
        print(f"{record['status'].upper()}: {name}", flush=True)
        if record["status"] == "failed":
            print(record["error"], file=sys.stderr, flush=True)

    for case in manifest["cases"]:
        if case["name"] not in selected:
            continue

        def operation(record: dict, case: dict = case) -> None:
            path, data = run_case(case, args.compiler, record, args)
            first_result = record["commands"][-1]
            if data is not None:
                record["wasm_sha256"] = hashlib.sha256(data).hexdigest()
            if args.compare_compiler:
                _, second = run_case(case, args.compare_compiler, record, args)
                if data != second:
                    raise Failure("Cargo and Blaze compiler builds produced different Wasm bytes")
                if case["kind"] in {"compile_error", "runtime_error"}:
                    second_result = record["commands"][-1]
                    if any(first_result[key] != second_result[key] for key in ("stdout", "stderr", "returncode")):
                        raise Failure("Cargo and Blaze compiler builds produced different failure diagnostics")
            if case["kind"] == "run":
                destination = args.output / "modules" / f"{case['name']}.wasm"
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, destination)
                browser.append({"name": case["name"], "wasm": str(destination), "stdout": case["stdout"]})

        perform(case["name"], operation)

    if "unicode_sources" in selected:
        perform("unicode_sources", lambda record: require_success(invoke(
            [sys.executable, str(ROOT / "tests" / "unicode" / "generate.py"), "--check"], record, args.timeout,
        )))
    if "migration_diagnostics" in selected:
        perform("migration_diagnostics", lambda record: migration_checks(record, args))
    if "runtime_values" in selected:
        perform("runtime_values", lambda record: require_success(invoke([
            sys.executable, str(ROOT / "crates" / "ilang-wasm" / "tests" / "runtime_values.py"),
            "--wasm-tools", args.wasm_tools, "--wasmtime", args.wasmtime,
        ], record, args.timeout)))
    if "host_abi" in selected:
        perform("host_abi", lambda record: browser.append(host_abi_check(record, args)))
    for corpus in manifest["corpora"]:
        if corpus["name"] in selected:
            perform(corpus["name"], lambda record, corpus=corpus: browser.append(run_corpus(corpus, record, args)))
    def verify_tool_identity(record: dict) -> None:
        for path, expected in report["tool_sha256"].items():
            if file_digest(Path(path)) != expected:
                raise Failure(f"executable changed during this run; rerun with stable binaries: {path}")
        if source_fingerprint() != report["source_sha256"]:
            raise Failure("compiler, stdlib or test sources changed during this run; rerun after changes finish")
    perform("tool_identity", verify_tool_identity)
    report["finished_utc"] = datetime.now(timezone.utc).isoformat()
    write_json(args.output / "report.json", report)
    print(f"{report['passed']} passed; {report['failed']} failed; report: {args.output / 'report.json'}")
    return int(report["failed"] != 0)


if __name__ == "__main__":
    raise SystemExit(main())
