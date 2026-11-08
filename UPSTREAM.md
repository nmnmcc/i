# Upstream sources

The i programming language (iLang) started from Gleam. Its parser, type checker, formatter and language server derive from the Gleam compiler, and its standard library derives from gleam_stdlib. i is now developed independently and is not compatible with Gleam.

| Source | Baseline | Commit | License |
| --- | --- | --- | --- |
| [Gleam](https://github.com/gleam-lang/gleam) | v1.19.2 | ed41eace2dd89054eb2d744c33925261d1eff32e | Apache-2.0 |
| [gleam_stdlib](https://github.com/gleam-lang/stdlib) | v1.0.5 | 5b0642a77a38f984db5913548b84668652ca8b5c | Apache-2.0 |

The derived compiler lives in `compiler/` and the derived standard library in `stdlib/`; both are maintained as part of i. Modified files are renamed or reorganised (for example `gleam/*` modules became `i/*`, sources use `.i`, manifests use `i.toml`). Original SPDX headers and copyright notices are retained. i additions are Apache-2.0 licensed. The Rust and JavaScript hosting SDKs are part of i; they do not replace the language's Wasm runtime.

Unicode algorithms use the pinned Unicode 17.0.0 data and its retained license;
the data inventory and regeneration instructions are in
[`stdlib/unicode`](stdlib/unicode).

The Devenv development environment selects
[Blaze v0.20261027.0](https://github.com/indexable-inc/blaze/tree/v0.20261027.0)
at `5d4f78f7577cf7170993b74f735099dd03f5c6ca`, licensed MIT OR Apache-2.0.
It includes the upstream cache-identity fix
[`7145b282f6c3adccee8f24b200cb0270eba477fc`](https://github.com/indexable-inc/blaze/commit/7145b282f6c3adccee8f24b200cb0270eba477fc)
as a recorded backport. The adapted patch and its MIT license are retained in
[`patches`](patches); the baseline compiler and nightly pins are unchanged.
See [development](docs/development.md) for the scoped Nix packaging corrections.
