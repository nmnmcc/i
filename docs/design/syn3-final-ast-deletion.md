# Applying the final-tree deletion

Preparation only (Nov14). The scripts have passed Python syntax checks and
the driver dry run; production deletion has not been applied or compiled.
They adapt lane/fa2 af804b08 to the native resolver, engine and final rows.
The complete step-three gate is a prerequisite. The coordinator's latest
instruction permits compiling milestone commits; a compiling deletion is
still not a green deletion until its full gate passes.

1. Commit and record the fully green step-three source. Create the clean
   `lane/fa-delete` branch in this worktree, merge stage12, preserve both the
   alias-aware provider fix and cached-row validation, and retain both sets
   of appended provider regressions. Check the merged workspace through the
   shared wrapper and commit the compiling integration milestone.
2. Capture the existing five-case engine comparison before removing the old
   hook. Both old and native answers must agree in the existing test. The
   capture helper adds one print probe, preserves every assertion, records
   the exact head and five source/answer pairs, then removes only its probe.
   Read and explain the answers: standalone type needs, expression grouping,
   generated aliases and previously fixed sum-absorption boundaries.

```sh
python3 scripts/syn3-capture-engine.py instrument
H=/home/agent/Programming/i/.rd/cache/heavy
$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test macro_run --locked type_macro_keeps_its_standalone_need_channel -- --exact --nocapture --test-threads=2 > target/fa-r3/capture-engine.log 2>&1
python3 scripts/syn3-capture-engine.py collect
```

3. Record `target/fa-night/step3-green.json` with `commit` equal to the
   current stage12 integration head and `oracle_ref` equal to the actual
   green step-three head. The pre-stage12 reference supplies only the private
   historical parser/tree needed by migration tools and compatibility tests.
   Confirm the worktree is clean.
4. Run `python3 scripts/syn3-delete-old-tree.py --apply`. The driver requires
   the exact proof, clean deletion branch and matching engine capture before
   changing files. Its eight helpers finish resolver, macro, source syntax,
   definitions, Lisp compatibility, historical assertions, comment APIs and
   the five-case engine oracle.
5. Resolve the new path dev-dependency lockfiles using one wrapped offline
   workspace check and then one wrapped standalone migration-package check
   without `--locked`; subsequent checks use `--locked`. Format both packages.
   Commit every compiling milestone and append the lane status line. Run the
   full coordinator matrix again, including all CLI binaries, both std
   levels, filtered spec chapters and short generation.

The old tree, parser, walker, renderer and converter live in the standalone
`crates/ilang-syntax/examples/legacy` migration package. Compiler production
dependencies never point to it. Existing node assertions and strict resolver
comparisons remain executable through dev-dependencies and cfg(test) modules.
The private lowerer can read native facts through a read-only metadata
adapter while retaining its own historical recording deltas. The CLI engine
comparison keeps all five inputs and complete printed-tree equality against
the captured matching old answers, rather than comparing only selected fields.

The original ten-thousand-level historical comment-renderer test is retained
unchanged. The ten-thousand-comment ordering test exercises the native API
with only the empty parameter data changed to `(,)`; a nested native-anchor
regression is also added. Design syntax-3 §6.8: “`(,)` is the empty data
wherever data is written, and so the empty parameter data. The empty Round
`()` has no legal position; `(f)` is a call with one part, not an empty Round.”

Do not infer that schema.null_round needs a backend patch: its unchanged
assertion passes all2170 std tests at both O0 and O2 before deletion. Repeat
both levels after deletion. The historical fa2 failure's cause is not known.
