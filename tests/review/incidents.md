# Verification and recovery notes

The lane started clean. There was no interrupted worker's uncommitted work to
keep or discard. Compiler and library changes in this branch come only from
the required merge of redesign's W520 commit; QA authored no compiler/library
change.

The following unsuccessful checks and recovery steps are retained:

- `verify.log`: the first full gate failed the formatter's fixed skip inventory
  after adding deliberately invalid review inputs. Parser-negative inputs now
  live in `.invalid`, with exact entries in the existing syntax registry. Valid
  inputs still take part in the original formatter properties. No formatter
  assertion or existing skip entry was weakened.
- `verify-final.log`: the next full gate found that the copied K4s std package
  had initially omitted modules whose bytes already existed in maintained
  tests. Import now keeps every module of a recovered package. The recaptured
  result matched the actual full-gate result (`std-recapture.log`).
- `verify-complete.log`: the nested-50 stress program finished its O2 build
  inside a three-second budget on a later run. Treating that as an unexpected
  semantic fix had introduced an accidental timing gate. Censored resource
  observations now report completion separately. Complete observations still
  compare in full; ordinary known-defect unexpected passes still fail.
- The first source import's cleanup encountered an already-removed nested
  package directory. Cleanup now visits deepest package directories first and
  checks whether they still exist. A complete reimport restored its outputs.
- `models.json`: the first optional model replay treated the historical
  forgetting-conflict witness as a passing model. Its own assertion documents
  the old inconsistent rules and must fail. `model-control.json` verifies that
  exact failure. The eager substitution model exceeded 120 seconds; it passed
  with a 600-second observation budget (`model-substitution.json`). BDD,
  demand and URI models passed in the initial replay.
- The same first replay resolved a virtual-environment Python symlink to the
  base interpreter, losing gmpy2. The adapter now retains the executable path.
  All four MPFR replays passed in `models-mpfr.json`: 254 main comparisons,
  59 extra comparisons, six midpoint ties and 12 underflow comparisons.
  The environment and the original review tools were not modified.
- `bench-inherited-rss.json`: direct wait4 measurements included the Python
  parent's inherited RSS floor for small children. These memory numbers are
  superseded. GNU time now measures a checker child after exec; the controls
  in `memory-control.json` distinguish the small true command and the compiler.
  The first control tried `/bin/true`, absent in this Nix environment; it was
  repeated with the executable found in the development shell.
- `bench-contended.json`: the first corrected-RSS recording overlapped the
  unwrapped startup-corpus recapture. Its numbers are retained as contended
  evidence. The final baseline is recorded after correctness work under the
  coordinator's quiet wrapper. A queued pre-W520 recording was superseded
  before collecting any samples; its own waiting processes/locks were released.
- W520 landed while QA was running. The lane merged it, rebuilt its compiler,
  retained all old observations in `baselines/pre-w520.json`, and recaptured
  the corpus. `w520-audit.json` checks all 18 changed diagnostic locations and
  shown types. It labels timeout and startup-harness changes separately.
- The first corpus runs compiled entryless libraries but executed zero tests.
  Accepted packages now receive a temporary ordinary Nil test so startup is
  exercised. The original source still supplies the shown-type baseline.
  `verify-startup.log` is the full gate for this stronger runner.

Interpretation checks are separate from implementation checks. The T1 model
reproduces all 180 original coverage decisions (`model-audit.json`). Generated
runtime assertions use its first-match results. The minimizer's negative
control intentionally used an always-accepting fake compiler and produced a
two-clause counterexample (`harness-control.json`); it is not a compiler defect.
The q24 review programs already accept on this baseline. Their alternate
printed scheme alone did not establish a principality defect, so they are
required positive regressions, with their full observed types retained.

The ledger names the accepted compiler prior-art note as landed at `3cc1def`,
but the starting redesign tree lacked it. QA read the exact note using git
show. Older K4 criteria demand E0522 from untargeted check; current spec §2.6.2
separates source checking from target validation. Required service rejection is
therefore before entry execution. An E0505 occurs rejection or a cast failure
does not establish that the service obligation was retained.

All 3,336 discovered i-source occurrences are accounted for in inventory.json.
Build products, virtual environments, node_modules and third-party Binaryen
sources are not corpus programs. Changed/novel drivers inside copied compiler
snapshots were preserved by a second source-only salvage pass. Historical
Rust/JavaScript mutation drivers and compiler variants are archived tools;
the normal runner uses the current public CLI, not those old binaries.

Initial generator scratch directories used the system temporary directory and
were automatically removed. Later scratch files stay under the lane. Build,
Node and ignored test caches are retained locally. No system configuration,
Git configuration, reference checkout, or another lane's source was changed.
