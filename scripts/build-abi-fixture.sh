#!/usr/bin/env bash
# Assemble the hand-written host ABI fixtures (tests/browser/fixtures/*.wat),
# validate them, and embed the feature probe in the JavaScript SDK.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_dir="$repo_root/tests/browser/fixtures"
sdk="$repo_root/bindings/javascript/ilang.mjs"
wasm_tools="${WASM_TOOLS:-wasm-tools}"

for fixture in host-abi required-features custom-imports; do
  "$wasm_tools" parse "$fixture_dir/$fixture.wat" -o "$fixture_dir/$fixture.wasm"
  "$wasm_tools" validate "$fixture_dir/$fixture.wasm"
done

# The SDK checks the engine with the probe, without its name section.
probe="$("$wasm_tools" strip "$fixture_dir/required-features.wasm" | base64 | tr -d '\n')"
sed -i.bak -E "s|^(const featureProbe = Uint8Array\.from\(atob\(\")[A-Za-z0-9+/=]*(\"\).*)$|\1$probe\2|" "$sdk"
rm -f "$sdk.bak"
grep -q "atob(\"$probe\")" "$sdk" || { echo "could not update the feature probe in $sdk" >&2; exit 1; }
