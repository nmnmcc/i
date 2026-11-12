# Simplify 01 report

The retired syntax migration was referenced by the syntax module, the old-syntax inventory, and two historical reports. Removed the module export, four inventory entries for deleted fixtures, and obsolete migration instructions and command history. The current parser and `scripts/old-syntax-check` remain.

`tests/spec/9.10/replay.py` builds and runs the `migrate-tests` example to extract test bodies, so that example and `scripts/test-macro-migrate` remain under the task's test-use exception.

The scheme wire tests retain their original text, with the enclosing module's indentation removed and one signature reflowed by Rust formatting. The implementation is unchanged.

`git diff --stat` before adding this report:

```text
 crates/ilang-check/src/types/scheme_wire.rs       | 1228 +-----------
 crates/ilang-check/src/types/scheme_wire/tests.rs | 1221 ++++++++++++
 crates/ilang-syntax/examples/migrate-syntax2.rs   |  427 -----
 crates/ilang-syntax/src/lib.rs                    |    1 -
 crates/ilang-syntax/src/parser_legacy.rs          | 2050 ---------------------
 docs/design/jp-step6-report.md                    |    3 -
 scripts/old-syntax-check.json                     |   24 -
 scripts/syntax-2-inline-migrate                   |   60 -
 scripts/syntax-2-migrate                          |  268 ---
 scripts/syntax-2-reviewed.json                    |  680 -------
 tests/syn2-stages.md                              |   10 -
 11 files changed, 1222 insertions(+), 4750 deletions(-)
```

Verification: `scripts/dev-verify rust` and `scripts/old-syntax-check` pass. Two runs of `scripts/dev-verify i` failed in the review corpus, including `review-k4s/std: unclassified checker timeout` and output changes for several stress cases. The compiler implementation is unchanged. Small reproduction (after building the release CLI):

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i scripts/review-corpus run --binary "$PWD/target/release/i" --filter review-k4s/std
```

This selected one case and failed with `review-k4s/std: full output changed` and `review-k4s/std: unclassified checker timeout`. The complete acceptance check therefore did not pass. The failure was not resolved in this lane.
