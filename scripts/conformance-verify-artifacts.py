#!/usr/bin/env python3
"""Prove a build-only compiler change preserves already-executed Wasm artifacts.

This is supplemental verification, not a replacement for full conformance. It
requires a complete successful execution report, rebuilds with caches disabled,
and rejects any differing Wasm byte or diagnostic. Corpus test bodies are NOT
executed again: their coverage is explicitly inherited from identical artifacts.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import sys

import conformance as suite


def verify_module_only() -> int:
    """CLI runner hook that reports artifact identity, never test execution."""
    expected = os.environ.get("WLEAM_ARTIFACT_EXPECTED_SHA256", "")
    if not re.fullmatch(r"[0-9a-f]{64}", expected):
        raise suite.Failure("artifact verifier requires its expected SHA-256")
    path = Path(sys.argv[1]).resolve()
    actual = suite.file_digest(path)
    if actual != expected:
        raise suite.Failure(f"rebuilt Wasm differs from executed reference: {path}: {actual} != {expected}")
    print(json.dumps({"mode": "artifact_equivalence", "runtime_tests_executed": 0,
                      "wasm_sha256": actual, "module": str(path)}))
    return 0


def diagnostic(event: dict) -> tuple:
    command = event["command"]
    project = command[command.index("--project") + 1]
    return (event["returncode"], event["stdout"].replace(project, "<project>"),
            event["stderr"].replace(project, "<project>"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", required=True, action="append", help="fresh compiler, repeat for Cargo and Blaze")
    parser.add_argument("--runner", required=True, help="unchanged, previously validated native runner")
    parser.add_argument("--reference-report", type=Path, required=True)
    parser.add_argument("--wasm-tools", default="wasm-tools")
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument("--output", type=Path, default=suite.ROOT / "target/verification/artifact-equivalence.json")
    args = parser.parse_args()
    args.compiler = [suite.executable(path) for path in args.compiler]
    args.runner = suite.executable(args.runner)
    args.wasm_tools = suite.executable(args.wasm_tools)
    args.output = args.output.resolve()
    args.reference_report = args.reference_report.resolve()
    reference = json.loads(args.reference_report.read_text())
    manifest = json.loads((suite.SUITE / "cases.json").read_text())
    golden = {record["name"]: record for record in reference["results"]}
    required = {item["name"] for item in manifest["cases"] + manifest["corpora"]}
    required.update({"host_abi", "runtime_values", "migration_diagnostics", "unicode_sources", "tool_identity"})
    if reference.get("failed") != 0 or set(golden) != required or any(r.get("status") != "passed" for r in golden.values()):
        parser.error("the reference must be a complete successful conformance execution report")
    if reference.get("tool_sha256", {}).get(args.runner) != suite.file_digest(Path(args.runner)):
        parser.error("the native runner must be exactly the executable recorded in the reference run")
    suite.check_upstream_sources()
    identity = {path: suite.file_digest(Path(path)) for path in [*args.compiler, args.runner]}
    report = {"schema": 1, "mode": "artifact_equivalence", "started_utc": datetime.now(timezone.utc).isoformat(),
              "verifier_sha256": suite.file_digest(Path(__file__)),
              "reference_report": str(args.reference_report), "reference_report_sha256": suite.file_digest(args.reference_report),
              "source_sha256": suite.source_fingerprint(), "tool_sha256": identity,
              "corpus_runtime_tests_executed": 0, "results": []}

    def perform(name: str, compiler: str, operation) -> None:
        record = {"name": name, "compiler": compiler, "commands": [], "status": "running"}
        try:
            operation(record)
            record["status"] = "passed"
        except (suite.Failure, OSError, ValueError, KeyError) as error:
            record.update(status="failed", error=str(error))
        report["results"].append(record)
        report["passed"] = sum(r["status"] == "passed" for r in report["results"])
        report["failed"] = sum(r["status"] == "failed" for r in report["results"])
        suite.write_json(args.output, report)
        print(f"{record['status'].upper()}: {Path(compiler).name}: {name}", flush=True)
        if record["status"] == "failed":
            print(record["error"], file=sys.stderr, flush=True)

    def identical(path: Path, name: str, record: dict) -> None:
        data = suite.validate_artifact(path, record, args)
        actual = hashlib.sha256(data).hexdigest()
        if actual != golden[name]["wasm_sha256"]:
            raise suite.Failure(f"{name}: changed Wasm requires complete runtime conformance: {actual}")
        record.update(wasm_sha256=actual, artifact_identity="identical_to_executed_reference")

    for compiler in args.compiler:
        for case in manifest["cases"]:
            def check_case(record: dict, case: dict = case) -> None:
                project = (suite.SUITE / case["project"]).resolve()
                command = lambda *parts: suite.compiler_command(compiler, project, args.runner, "--no-cache", *parts)
                checked = suite.invoke(command("check"), record, args.timeout)
                if case["kind"] == "compile_error":
                    suite.require_failure(checked, case["stderr_pattern"])
                    if diagnostic(checked) != diagnostic(golden[case["name"]]["commands"][0]):
                        raise suite.Failure("compile-error diagnostic differs from executed reference")
                    return
                suite.require_success(checked)
                suite.remove_artifact(suite.artifact(project))
                suite.require_success(suite.invoke(command("build"), record, args.timeout))
                identical(suite.artifact(project), case["name"], record)
                if case["kind"] == "runtime_error":
                    result = suite.invoke(command("run"), record, args.timeout)
                    suite.require_failure(result, case["stderr_pattern"])
                    if diagnostic(result) != diagnostic(golden[case["name"]]["commands"][-1]):
                        raise suite.Failure("runtime-error diagnostic differs from executed reference")
                    record["runtime_diagnostic_probes_executed"] = 1
            perform(case["name"], compiler, check_case)

        def check_host(record: dict) -> None:
            project = suite.SUITE / "projects/host_abi"
            suite.remove_artifact(suite.artifact(project))
            suite.require_success(suite.invoke([compiler, "--project", str(project), "--no-cache", "build", "--lib"], record, args.timeout))
            identical(suite.artifact(project), "host_abi", record)
            if json.loads(suite.artifact(project).with_suffix(".abi.json").read_text()) != json.loads((project / "expected-abi.json").read_text()):
                raise suite.Failure("host ABI companion descriptor changed")
        perform("host_abi", compiler, check_host)

        for corpus in manifest["corpora"]:
            def check_corpus(record: dict, corpus: dict = corpus) -> None:
                with suite.corpus_project(corpus) as project:
                    suite.remove_artifact(suite.artifact(project, tests=True))
                    names = suite.corpus_test_names(corpus, project)
                    if len(names) != corpus["expected_tests"]:
                        raise suite.Failure("corpus inventory changed")
                    env = dict(os.environ, WLEAM_ARTIFACT_EXPECTED_SHA256=golden[corpus["name"]]["wasm_sha256"])
                    command = [compiler, "--project", str(project), "--no-cache", "--json", "test", "--runner", str(Path(__file__).resolve())]
                    result = suite.invoke(command, record, args.timeout, env=env)
                    suite.require_success(result)
                    proof = json.loads(result["stdout"])
                    if proof.get("mode") != "artifact_equivalence" or proof.get("runtime_tests_executed") != 0:
                        raise suite.Failure("compiler did not invoke the explicit artifact-only verifier")
                    path = suite.artifact(project, tests=True)
                    identical(path, corpus["name"], record)
                    if set(json.loads(path.with_suffix(".abi.json").read_text())["tests"]) != names:
                        raise suite.Failure("compiled corpus ABI test inventory changed")
                    record.update(referenced_runtime_tests=corpus["expected_tests"], runtime_tests_executed=0)
            perform(corpus["name"], compiler, check_corpus)

    def verify_inputs(record: dict) -> None:
        if suite.source_fingerprint() != report["source_sha256"]:
            raise suite.Failure("source changed during equivalence verification")
        if any(suite.file_digest(Path(path)) != expected for path, expected in identity.items()):
            raise suite.Failure("compiler or runner changed during equivalence verification")
        if suite.file_digest(args.reference_report) != report["reference_report_sha256"]:
            raise suite.Failure("reference execution report changed")
        if suite.file_digest(Path(__file__)) != report["verifier_sha256"]:
            raise suite.Failure("artifact verifier changed during execution")
    perform("input_identity", args.compiler[0], verify_inputs)
    report["finished_utc"] = datetime.now(timezone.utc).isoformat()
    suite.write_json(args.output, report)
    print(f"{report['passed']} passed; {report['failed']} failed; corpus runtime tests executed: 0; {args.output}")
    return int(report["failed"] != 0)


if __name__ == "__main__":
    try:
        if len(sys.argv) > 1 and sys.argv[1].endswith(".wasm"):
            raise SystemExit(verify_module_only())
        raise SystemExit(main())
    except (suite.Failure, OSError, ValueError, KeyError) as error:
        print(f"artifact equivalence failed: {error}", file=sys.stderr)
        raise SystemExit(1)
