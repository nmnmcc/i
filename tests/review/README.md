Permanent independent-review regression inputs. `manifest.json` records each
project's origin and controlling rule; `inventory.json` accounts for every
source occurrence, including exact copies already in the repository.

Use `scripts/review-corpus run` for full shown-type, diagnostic, O0/O2 test and
main-output comparisons. `expected.json` is an observed baseline, not evidence
that the compiler follows the language. `requirements.json` separately records
known defects and the required result; an unexpected pass requires promotion.
Accepted temporary packages include one ordinary Nil test to exercise startup,
including source libraries with no original entry. Censored observations keep
their raw partial outputs; completed phases still compare exactly.

The review drivers, logs and compiler snapshots that came with the cases are
not kept here; they are in git history (removed after commit d893698a). The
finite models that `scripts/test-generation --models` replays are
`tests/t3-*.py` and `scripts/qa/models/`. Use the portable
`scripts/test-generation` adapters for ongoing generation.
