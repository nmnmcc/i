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

`tools/` preserves original review generators, finite models and evidence.
Historical tools contain absolute paths and mutation commands: they are source
artifacts, not commands to run. Host drivers have `.source` suffixes. Use the
portable `scripts/test-generation` adapters for ongoing generation.
