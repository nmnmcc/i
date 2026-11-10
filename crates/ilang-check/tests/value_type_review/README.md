# Independent T2 review programs

These are the review's complete source projects, retained as authored. The
manifest `cases.tsv` names every project and its required diagnostics; an empty
second column requires acceptance. `independent_review_reproducers` checks them
through the ordinary package loader and compiler, and through query recovery.
The three pkg projects keep their relative path dependencies inside this tree.

Interpretation requirements (not generated expectations):

- P4/spec 4.3 requires same-head normalization, nested/partial extraction, exact
  constant equality, no discarded union member, and rejection of mismatch.
- P1-P3 requires the written-head classification, ordinary type lookup and
  arity, unique binders/formals, and transparent qualified aliases. A bare
  existing name still cannot be rebound. V2/V3 governs shadowing/pending names.
- R1-R5/spec 4.7 requires fresh generalized annotation instances, shared local
  and weak parts, own-module checked fixation and precise fixation locations;
  generalized/unfilled permanent reads and earlier weak dependencies fail.
- R7 requires whole-source closure for foreign reads before extraction,
  including skipped arguments; foreign_alias and the pkg chain use the
  exported completed alias instead.
- O1/O3/O4 requires cycle errors and unrelated-error/query recovery. X1/X2
  requires export privacy and closed public aliases. J6 requires early-context
  rejection. The user's annotation-demand ruling requires weak_anno to pass.
- X4 requires nested source/head/binder navigation; repository query tests make
  specific assertions on nested/main.i's shape. Runtime's four assertions are
  also copied into spec/4.3/value_type_review_runtime, requiring both O0/O2.

`review_declaration_permutations_are_identical` generates all 120 permutations
from the review's five declarations. The CLI performance test generates every
ordinary/value-dependent chain and reverse chain at all four requested sizes;
no scratch-directory files or external timing scripts are required.
