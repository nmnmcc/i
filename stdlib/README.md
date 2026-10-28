# Wleam standard library

This is the WasmGC port of [Gleam stdlib v1.0.5](https://github.com/gleam-lang/stdlib/releases/tag/v1.0.5), based on upstream commit `5b0642a77a38f984db5913548b84668652ca8b5c`. It retains the upstream public API and Apache-2.0 [licence](LICENCE). Wleam bundles these sources and makes the `gleam/*` modules available to projects automatically.

```gleam
import gleam/io

pub fn main() -> Nil {
  io.println("Hello from Wasm!")
}
```

The library targets Core Wasm with garbage collection, typed function references, and tail calls. Collections, Unicode processing, numeric algorithms, binary encoding, and URI handling execute in Wasm. JavaScript and Erlang native implementations have been removed from the active source tree.

Dictionaries use a persistent hash array mapped trie. Integers have arbitrary precision and signed bitwise operations. Strings use UTF-8, Unicode 17.0.0 extended grapheme segmentation, and default full case conversion. Bit arrays retain their exact bit length, including non-byte-aligned values. Float parsing and formatting use exact integer arithmetic; transcendental functions use portable range reduction.

`exp`, `log`, and `power` use the same implementation on every Wasm host. Their results can differ in the last bit from an Erlang platform's native math library. The port's 60,042-case reference comparison observed a maximum difference of one ULP; this is a measured result, not an error bound for every possible input.

Dynamic decoding works with Wleam values, dictionaries, and `dynamic.array` values. Foreign objects are opaque host references; encode their fields explicitly before applying structural decoders.

The internal imports recorded in [INTRINSICS.json](INTRINSICS.json) belong to the compiler's reserved `wleam:runtime/v1` namespace. They use the compiler's boxed representation and are resolved inside the emitted Wasm module. They are separate from application host imports. Console output and random-number entropy are provided through the host adapter.

Run the migrated upstream tests with:

```sh
wleam --project stdlib test
```

The [test migration ledger](test/migration.json) records the treatment of platform-specific upstream cases. Additional regression fixtures live in `../tests/stdlib`, and Unicode reference data and regeneration instructions live in [unicode](unicode). The project conformance harness runs these fixtures in addition to the upstream corpus and checks the same emitted Wasm across supported hosts.
