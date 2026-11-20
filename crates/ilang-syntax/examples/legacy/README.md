Private old tree, readers and test-span inspector, kept as the test oracle for
lowering and the cross-target replay. This is a
standalone Cargo package, outside the compiler workspace. Build with the shared
heavy wrapper and `cargo check --manifest-path crates/ilang-syntax/examples/legacy/Cargo.toml --all-targets`.
The compiler has no production dependency on this package; its query index test uses it as an oracle.
Production Datum, span and token types remain shared.
