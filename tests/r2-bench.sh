#!/usr/bin/env bash
# Run from the R2 lane through the heavy wrapper and dev shell. The executable
# must be the release compiler built by scripts/bench or dev-verify i.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
for sample in 1 2 3 4 5; do
  echo "sample $sample"
  target/release/i test --project bench --opt 2 --time r2_
done
