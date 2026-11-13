Private old tree, readers and converter for migration tools. This is a
standalone Cargo package, outside the compiler workspace. Build with the shared
heavy wrapper and `cargo check --manifest-path crates/ilang-syntax/examples/legacy/Cargo.toml --all-targets`.
The compiler has no production dependency on this package; its query index test uses it as an oracle. The migration implementation is
preserved from the green step-three oracle recorded by the deletion driver. Production Datum, span and token types remain shared.
