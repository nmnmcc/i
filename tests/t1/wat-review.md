# T1 final WAT review

The final generation command is `WAT_DUMP=target/t1/continued/wat-final
cargo test -p ilang-cli --test wat_dump --locked`, through the prescribed
heavy/dev-shell wrapper. Cargo runs this test in the CLI crate, so the
832 generated files were initially under
`crates/ilang-cli/target/t1/continued/wat-final`, then moved unchanged to
`target/t1/continued/wat-final`; the now-empty generated directories were
removed. Generation passed. This
is an export comparison, separate from executing spec tests at O0/O2.

| Baseline | Same | Changed | Missing | Added |
|---|---:|---:|---:|---:|
| Supplied 1df2fe0 (522 files) | 188 | 334 | 0 | 310 |
| Earlier own 602-file checkpoint | 219 | 383 | 0 | 230 |
| Upstream 7b3cea3 (636 files, borrowed archive) | 627 | 9 | 0 | 196 |

The closest archive is read-only from the T2 lane's
`target/t2-baseline-wat-7b3cea3`. Its comparison report establishes the
636-file upstream baseline independently of T2's additions. This is a
borrowed baseline, not a compiler build or test performed by T1. The older
baselines include intervening B/C, opaque identity and random-library
changes; their larger residuals are not attributed to literal types.
The full byte-comparison inventory is retained at
`target/t1/continued/final-wat-comparison.json`.

The nine closest-baseline differences are:

- `11/equality`, `6/blocks`, `6/strings` with tests, and
  `11/stop_tail_service` on both targets: the §8-required parameter
  wrappers preserve runtime equality and stop checks. The last case's
  residual is its changed source column in site_data. Blocks also moves
  constant subjects behind parameters as §8 requires for W0501.
- `bench` and `std`, with and without tests on the i target: added Std-2
  declarations add Date, TimeOfDay, Offset, Civil and other constructor
  metadata and renumber the corresponding table references. Std with
  tests also contains the added calendar/cli/log/time tests. These compare
  different source inventories; they are not used as proof of compiler
  equivalence. The unused-constructor metadata cost is the coordinator's
  existing K3 follow-up, not changed here.

All remaining 627 common exports are byte-identical. The added files are
new T1/upstream fixtures, including fitted choices and literal tests;
existing exports do not disappear. All changed runtime fixtures and the
expanded std/bench corpus pass the real runtime gates.

An optional Python unified-diff review was interrupted on the large std
aggregate after the byte inventory and eight smaller diffs had completed.
GNU diff produced the remaining std diff as a substitute view; this did
not replace the byte comparison or any runtime check. Raw diffs remain
under `target/t1/continued/*.wat.diff`. No external process was signalled.

After the final binding-grants and Std-3a merges, export was repeated to
`target/t1/continued/wat-final-bg`: 832 files, no missing exports. Against
the same 636-file upstream archive the counts remain 627 identical and
nine changed. Against the preceding T1 export, 824 are identical and
eight differ: the four std/bench aggregates and four exports of two new
T1 fixtures importing std. The added property declarations change that
source inventory and its constructor metadata. This final inventory is
`target/t1/continued/latest-wat-comparison.json`; the earlier table is a
checkpoint comparison, not a replacement for the final export.
