#!/usr/bin/env bash
# Assemble the hand-written host ABI fixtures (tests/browser/fixtures/*.wat),
# validate them, and embed the feature probe in the JavaScript SDK. Then build
# tests/browser/fixtures/compiled.wasm with the compiler, from the package in
# tests/browser/fixtures/compiled.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_dir="$repo_root/tests/browser/fixtures"
sdk="$repo_root/bindings/javascript/ilang.mjs"
wasm_tools="${WASM_TOOLS:-wasm-tools}"

for fixture in host-abi required-features custom-imports one-thread no-continue wrong-types start-request bg-lifecycle; do
  "$wasm_tools" parse "$fixture_dir/$fixture.wat" -o "$fixture_dir/$fixture.wasm"
  "$wasm_tools" validate "$fixture_dir/$fixture.wasm"
done

# Every place that names the ABI version must say what the compiler writes.
compiler_version="$(sed -nE 's/^ *"version": ([0-9]+),$/\1/p' "$repo_root/crates/ilang-wasm/src/lower.rs")"
[ -n "$compiler_version" ] || { echo "could not read the ABI version from crates/ilang-wasm/src/lower.rs" >&2; exit 1; }
version_differs=0
check_version() {
  local file="$1" pattern="$2" found
  found="$(sed -nE "s/.*$pattern.*/\1/p" "$repo_root/$file" | head -n 1)"
  if [ "$found" != "$compiler_version" ]; then
    echo "$file says ABI version ${found:-(none)}, but the compiler writes $compiler_version" >&2
    version_differs=1
  fi
}
check_version crates/ilang-host/src/lib.rs 'pub const ABI_VERSION: u32 = ([0-9]+);'
for fixture in host-abi one-thread no-continue wrong-types start-request; do
  check_version "tests/browser/fixtures/$fixture.wat" '\{\\"version\\":([0-9]+),'
done
check_version tests/browser/fixture-abi.mjs 'version: ([0-9]+),'
check_version bindings/javascript/ilang.mjs 'const version = ([0-9]+);'
check_version bindings/javascript/ilang.d.ts 'version: ([0-9]+);'
[ "$version_differs" = 0 ] || exit 1

# The SDK checks the engine with the probe, without its name section.
probe="$("$wasm_tools" strip "$fixture_dir/required-features.wasm" | base64 | tr -d '\n')"
sed -i.bak -E "s|^(const featureProbe = Uint8Array\.from\(atob\(\")[A-Za-z0-9+/=]*(\"\).*)$|\1$probe\2|" "$sdk"
rm -f "$sdk.bak"
grep -q "atob(\"$probe\")" "$sdk" || { echo "could not update the feature probe in $sdk" >&2; exit 1; }

# The module built by the compiler (`I` can point at another `i` executable).
if [ -n "${I:-}" ]; then
  "$I" build --target host --project "$fixture_dir/compiled" legacy --out "$fixture_dir/compiled.wasm"
else
  cargo run --quiet -p ilang-cli -- build --target host --project "$fixture_dir/compiled" legacy --out "$fixture_dir/compiled.wasm"
fi
"$wasm_tools" validate "$fixture_dir/compiled.wasm"

# Structured concurrency runs on the identical O0/O2 artifacts in both JS hosts.
for level in 0 2; do
  if [ -n "${I:-}" ]; then
    "$I" build --target host --project "$fixture_dir/r2" --opt "$level" --out "$fixture_dir/r2-o$level.wasm"
  else
    cargo run --quiet -p ilang-cli -- build --target host --project "$fixture_dir/r2" --opt "$level" --out "$fixture_dir/r2-o$level.wasm"
  fi
  "$wasm_tools" validate "$fixture_dir/r2-o$level.wasm"
done

# Compile the hand-authored task/request runtime cases into shared O0/O2
# conformance modules. Expectations are copied from their source headers.
R2_FIXTURE_DUMP="$fixture_dir/r2-laws" cargo test --quiet -p ilang-cli --test r2_fixtures --locked -- generate_conformance_fixtures
for level in 0 2; do
  "$wasm_tools" validate "$fixture_dir/r2-laws/laws-o$level.wasm"
done
