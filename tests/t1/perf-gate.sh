#!/usr/bin/env bash
# Usage: perf-gate.sh <before release i> <after release i> <report directory> [cpu=6]
# Reserve the shared quiet window for timings, then release it before
# the debug-counter tests acquire an ordinary heavy-job slot.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd -- "$repo_root"
before="$1"
after="$2"
report_dir="$3"
cpu="${4:-6}"
shared_cache=/home/agent/Programming/i/.rd/cache
mkdir -p -- "$report_dir"
"$shared_cache/quiet" direnv exec /home/agent/Programming/i tests/t1/time-std.sh "$before" "$after" --cpu "$cpu" --require-quiet --output "$report_dir/std.json"
"$shared_cache/quiet" direnv exec /home/agent/Programming/i tests/t1/stress-time.sh "$before" "$after" --cpu "$cpu" --require-quiet --output "$report_dir/stress.json"
"$shared_cache/heavy" direnv exec /home/agent/Programming/i cargo test -p ilang-check --lib --locked t1_stress_ -- --ignored --nocapture --test-threads=1 > "$report_dir/ignored.log" 2>&1
cat -- "$report_dir/ignored.log"
