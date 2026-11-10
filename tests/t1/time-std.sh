#!/usr/bin/env bash
# Usage: time-std.sh <before release i> <after release i> --output <json> [--cpu N]
set -euo pipefail
exec python3 "$(dirname -- "${BASH_SOURCE[0]}")/perf.py" std "$@"
