#!/usr/bin/env bash
# Usage: stress-time.sh <before i> <after i> --output <json> --ignored-tests
set -euo pipefail
exec python3 "$(dirname -- "${BASH_SOURCE[0]}")/perf.py" stress "$@"
