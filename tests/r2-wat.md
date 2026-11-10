# R2 WAT audit

Reference: the 522 exports of 1df2fe0 in .rd/cache/wat/1df2fe0. The command
`WAT_DUMP=<lane>/target/r2-wat cargo test -p ilang-cli --test wat_dump --locked`
exports both targets and test modes. It runs through the heavy wrapper and
the dev shell. The first invocation used a relative dump path; the test's
working directory placed it under crates/ilang-cli/target instead of the lane's
root target. That was a local artifact-path incident, not a source change.
The directory is moved under the root target after analysis, leaving no extra
tracked artifact or changed environment setting.

The completed export command passed, exit 0, and produced 990 files. Against
the reference, 188 shared files are byte-identical ERROR results and all 334
shared compiling files changed. No shared export disappeared, newly failed or
changed from ERROR to compiling. There are 468 added exports from later
compiler/library/spec cases and R2's task/request/key/retention cases. Thus a
passing runtime test is not being confused with a successful WAT compilation
of a reference ERROR file.

The baseline precedes B, R1, P1, T1/T2/FP and Std additions as well as R2.
The differences have these source causes:

| Region | Cause and required behavior |
| --- | --- |
| frame/flist/sub types and re-entry functions, capture records at calls | B's lazy re-entry changes the saved-slot field and replaces recursive re-entry with the trampoline; inherited from redesign, tested by control_lazy |
| key/keyed/request object kinds and equality/hash/text/type/portable runtime | S7 §§3–4 and runtime §§3/5 require identity, invariant typed storage and the ordinary portable walk, including cycles |
| patched recursive closure captures | The portable walk adds a mutation flag and one walk-stamp tail slot; existing capture indices retain their positions |
| hostref type and constructors/ABI reply decoder | P1 requires per-declaration nominal tags; R2's request cell retains ImportReply.result for each pending call, tested with out-of-order opaque replies |
| host/request globals, functions and resume/continue exports | runtime §5 replaces the single outstanding request cell with typed cells, free slots, observers and Root/Owner accounting; ABI stays v3 |
| async import adapter and entry wrappers/metadata | Raw starts return cells; async adapters call ordinary await_owned/from_result. Only the R2 default-plan specialization gets ordinary request/task entry wrappers (coordinator ruling) |
| environment slot operands and initial array size | Root/Owner/Await and Scheduler/Runner enter the service type numbering. Assert stays 0; existing services keep their identity with shifted dense indices |
| constructor table and nominal ids, including programs using no tasks | The existing compiler initializes names for unused library types (ledger K3, also accepted for R1). Request/key/task/random add ordinary constructors and shift following user ids |
| top-level initializer globals, data ids, function ids and elem declarations | R2 adds ordinary identity-bearing constants and private helper functions; Std additions add their own constants. Numbering follows reachable functions and initialization order |
| std/bench source functions and added tests | Inherited Std-1/2/3 implementations and B's control benchmark; R2 adds the three named task benchmarks. These are source additions, not runtime renumbering |
| source-site data/comments | New library functions, wrappers and merged source lines add/shift positions; user call positions remain covered by spec trace tests |

Hello is a useful independent example: its ordinary main still directly calls
print and remains synchronous. After resolving function/data identifiers and
source sites, its main differs only in the Io environment index, 1 to 4.
Its constructor-name array grows from 1 to 120 entries. The unused task
empty_relay constant also contributes a helper function through ordinary
initialization. This is the existing K3 behavior; R2 does not claim unused
library definitions have zero binary-size overhead.

For reproduction, `python3 tests/r2-wat-audit.py REFERENCE DUMP OUTPUT` writes
an inventory and function-owner diffs. This is a **substitute diagnostic
comparison**, not a successful stock scripts/wat-diff run. The stock normalizer
rejects synthetic -1 site sentinels (ledger K3) and did not recognize the new
ordinary entry-wrapper comments. The diagnostic explicitly substitutes those
sentinels and identifies the wrapper owners. It also isolates B capture/frame
representation changes and the extra recursive-closure walk stamp. These
substitutions do not prove those runtime implementations; their independent
control, cyclic-portability, request and third-resume tests do that.

The ordinary-body residual review found:

- Existing spec code has no additional operation change after dense service,
  constructor, function/global/data/site and local point-label numbering are
  resolved, and the separately attributed B representations are isolated.
- Eight new top-level values precede old globals: seven typed keys and the
  private empty-active record. This accounts for old gN becoming g(N+8).
- Constructor-as-function hashes change because the hash contains the shifted
  ctor/type operands. The references retain their old source constructor and
  identity relationship, rather than changing to another value.
- Local `$okN`/`$capN` labels carry global point numbers and shift when preceding
  functions gain points. Their lexical targets and call operands are unchanged.
- The remaining std/string bodies differ by the merged Std-3 source positions
  (+20 lines for the old functions), not additional operations. New string
  functions and property modules are inherited source additions.
- Three old std/list_test bodies (collect, generator_callbacks_test and
  environment_callbacks_test) additionally have service indices shifted by the
  new std/log Sink along with R2 services. They still bind/read the same test
  services; the full i tests at O0/O2 check their callbacks and environment.
- Async adapters are deliberate operation changes at the import boundary:
  start a typed cell, await_owned it through Await, then decode/from_result.
  Ordinary user calls to those fields retain their original source positions.
  Resume stores a per-cell decoded answer before entering the one host k.
- Every initializer gains the eight R2 values; std additionally gains its
  merged top-level values. Existing initializer calls retain their source order.
  For example, the external-start/order and stop-in-start spec cases still pass.

No unexplained residual was found. This is an explanation of differences, not
a byte-equality claim or a claim that the older ERROR files have been fixed.

Retention inspection used the kept_services WAT and the O0/O2 r2 fixture
binaries, printed with wasm-tools. At O0, control_keep_env copies all slots to
ordinary tuple fields, keep_run calls under a surviving capture handler, and
keep_reenter resumes against that saved base. The execute/scope/sync wrappers
retain the call before their first suspension. At O2, names are stripped, but
the fixture's functions 70–72 preserve the corresponding operations: function
70 copies the complete environment into the closure tuple, 71 restores that
array and performs a non-tail call_ref under try_table, and 72 resumes with
the saved environment and packs that same base on another capture. The
optimizer has not removed the surviving handler frame. These numbers identify
this artifact only. The kept_services law exercises unused services and the
third resume independently at O0/O2 on Rust, Node and all three browsers.

The final pre-merge export, target/r2-wat-complete, passed in 277.70 s and has
1,010 files. The reference comparison remains exactly 188 byte-equal ERROR
files and 334 changed compiling files, with no missing export/error transition;
488 files are added. Against the first R2 snapshot, 875 shared files are
byte-identical. The other 115 are the 114 R2 task-case exports plus bench's
ilang/tests export. Removing only i/task.loop and its generated re-entry/join
functions makes all 114 task files byte-equal. Bench then differs only in its
site-data segment: decoding all 176 entries shows 23 changed coordinates, all
in the new r2_task_test module after formatting. Existing bench/std modules'
entries are unchanged. The first dump overlapped that source formatting; the
last dump uses the stable formatted source. No behavioral expectation was
produced from that intermediate snapshot.

Thus the additional operation change is exactly the §6.4 go progress-poll fix
and its generated continuation code. It adds the existing-interest guard and
counts a retained current task as runnable, while preserving the current turn.
The 20 additional exports are five late laws at four dump configurations each:
derived_seed, public_schemes, invalid_losing_join, invalid_losing_gone and
shielded_final_io. No source code or test was changed to hide a WAT residual.
