#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_dir="$repo_root/tests/browser/fixtures"
wasm_tools="${WASM_TOOLS:-wasm-tools}"

for fixture in host-abi required-features custom-imports; do
  "$wasm_tools" parse "$fixture_dir/$fixture.wat" -o "$fixture_dir/$fixture.wasm"
  "$wasm_tools" validate "$fixture_dir/$fixture.wasm"
done
