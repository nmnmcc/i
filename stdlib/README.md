# The i standard library

The i standard library started as a WasmGC port of [gleam_stdlib v1.0.5](https://github.com/gleam-lang/stdlib/releases/tag/v1.0.5), based on upstream commit `5b0642a77a38f984db5913548b84668652ca8b5c`. It is distributed under the Apache-2.0 [licence](LICENCE). i bundles these sources and makes the `i/*` modules available to projects automatically.

```i
import i/io

pub fn main() -> Nil {
  io.println("Hello from Wasm!")
}
```

The library targets Core Wasm with garbage collection, typed function references, and tail calls. Collections, Unicode processing, numeric algorithms, binary encoding, and URI handling execute in Wasm. JavaScript and Erlang native implementations have been removed from the active source tree.

Dictionaries use a persistent hash array mapped trie. Integers have arbitrary precision and signed bitwise operations. Strings use UTF-8, Unicode 17.0.0 extended grapheme segmentation, and default full case conversion. Bit arrays retain their exact bit length, including non-byte-aligned values. Float parsing and formatting use exact integer arithmetic; transcendental functions use portable range reduction.

`exp`, `log`, and `power` use the same implementation on every Wasm host. Their results can differ in the last bit from an Erlang platform's native math library. The port's 60,042-case reference comparison observed a maximum difference of one ULP; this is a measured result, not an error bound for every possible input.

Dynamic decoding works with iLang values, dictionaries, and `dynamic.array` values. Foreign objects are opaque host references; encode their fields explicitly before applying structural decoders.

The internal imports recorded in [INTRINSICS.json](INTRINSICS.json) belong to the compiler's reserved `ilang:runtime/v1` namespace. They use the compiler's boxed representation and are resolved inside the emitted Wasm module. They are separate from application host imports. Console output and random-number entropy are provided through the host adapter.

Run the migrated upstream tests with:

```sh
i --project stdlib test
```

The [test migration ledger](test/migration.json) records the treatment of platform-specific upstream cases. Additional regression fixtures live in `../tests/stdlib`, and Unicode reference data and regeneration instructions live in [unicode](unicode). The project conformance harness runs these fixtures in addition to the upstream corpus and checks the same emitted Wasm across supported hosts.
