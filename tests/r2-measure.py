#!/usr/bin/env python3
"""Build, measure under quiet, then count the actual O2 code separately.

Run in the lane: direnv exec /home/agent/Programming/i python3
tests/r2-measure.py target/r2-measurement
"""
import csv
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parent.parent
os.chdir(root)
out = pathlib.Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=True)
cache = pathlib.Path("/home/agent/Programming/i/.rd/cache")
shell = ["direnv", "exec", "/home/agent/Programming/i"]
frozen = {name: (root / name).read_bytes() for name in [
    "lib/i/task.i", "tests/r2-performance/src/kernels.i",
    "tests/r2-performance/intervals.i", "crates/ilang-cli/tests/r2_performance.rs"]}
(out / "source-hashes.json").write_text(json.dumps({
    name: hashlib.sha256(data).hexdigest() for name, data in frozen.items()}, indent=2) + "\n")
(out / "task-source.i").write_bytes(frozen["lib/i/task.i"])


def unchanged():
    assert all((root / name).read_bytes() == data for name, data in frozen.items()), "measurement source changed"



def run(args, **kwargs):
    return subprocess.run(list(map(str, args)), check=True, **kwargs)


build = run([cache / "heavy", *shell, "cargo", "test", "-p", "ilang-cli", "--test",
             "r2_performance", "--locked", "--no-run", "--message-format=json"],
            capture_output=True, text=True)
unchanged()
(out / "build.jsonl").write_text(build.stdout)
artifacts = [json.loads(line) for line in build.stdout.splitlines()]
binary = next(item["executable"] for item in artifacts
              if item.get("executable") and item.get("target", {}).get("name") == "r2_performance"
              and "test" in item["target"]["kind"])
archive = max(root.glob("target/debug/build/ilang-binaryen-*/out/lib64/libbinaryen.a"),
              key=lambda path: path.stat().st_mtime)
helper = out / "profile-opt"
run([cache / "heavy", *shell, "cc", "tests/r2-profile-opt.c", "-I",
     "crates/ilang-binaryen/binaryen/src", archive, "-lstdc++", "-lm", "-lpthread",
     "-o", helper])
env = os.environ.copy()
env.pop("R2_PROFILE_WASM", None)
env["R2_PERF_DUMP"] = str(out / "outer")
env["R2_PERF_INTERVAL_DUMP"] = str(out / "inside")
run([cache / "quiet", *shell, binary, "--test-threads=1", "--nocapture"], env=env)
unchanged()

p = out / "outer"
run([cache / "heavy", *shell, helper, p / "kernels-o0.wasm", p / "named-o2.wasm"])
for name in ["kernels", "named-o2"]:
    run(["wasm-tools", "strip", "--all", p / f"{name}.wasm", "-o", p / f"{name}-stripped.wasm"])
    run(["wasm-tools", "print", p / f"{name}-stripped.wasm", "-o", p / f"{name}-stripped.wat"])
    run(["wasm-tools", "print", p / f"{name}.wasm", "-o", p / f"{name}.wat"])
a = (p / "kernels-stripped.wat").read_text()
b = (p / "named-o2-stripped.wat").read_text()
assert a[a.index("  (func "):a.index("  (data ")] == b[b.index("  (func "):b.index("  (data ")]
(p / "attribution.txt").write_text("All optimized function bodies match the timed artifact exactly.\n")
run(["python3", "tests/r2-profile.py", p / "kernels.wat", p / "counted.wat",
     p / "counted.wasm.tsv", p / "named-o2.wat"])
run(["wasm-tools", "parse", p / "counted.wat", "-o", p / "counted.wasm"])
env = os.environ.copy()
env["R2_PROFILE_WASM"] = str(p / "counted.wasm")
run([cache / "heavy", *shell, binary, "count_instrumented_kernels", "--nocapture"], env=env)

# Save uninstrumented hot functions, with the separately measured entry counts.
named = (p / "named-o2.wat").read_text()
numbers = {name: int(number) for name, number in re.findall(
    r"^  \(func (\$\S+) \(;([0-9]+);\)", named, re.M)}
blocks = {}
for match in re.finditer(r"^  \(func \(;([0-9]+);\)", a, re.M):
    end = a.index("\n  )", match.start()) + len("\n  )")
    blocks[int(match[1])] = a[match.start():end]
rows = list(csv.DictReader((p / "counted.wasm.counts.tsv").open(), delimiter="\t"))
for kernel in ["empty", "spawn_join", "switch_pair", "channel_pair"]:
    hot = sorted((row for row in rows if row["kernel"] == kernel and row["n"] == "128"
                  and row["kind"] == "entry"), key=lambda row: -int(row["count"]))[:12]
    (p / f"{kernel}-hot.wat").write_text("\n\n".join(
        f';; {row["function"]}: {row["count"]} entries\n' + blocks[numbers[row["function"]]]
        for row in hot))
unchanged()
print(f"Raw paired samples, counters and uninstrumented hot WAT: {out}")
