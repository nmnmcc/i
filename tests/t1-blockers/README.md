# T1 source conflict: the empty optional accumulator

The required unannotated `None` accumulator example in literal-types §10
cannot run under that same note's known-type rules. This is a source
conflict, not permission to weaken the check. No language rule or accepted
design text has been changed to resolve it.

The reduced program is in `none-accumulator/src/main.i`. Its manifest uses
this lane's std. It preserves the §10 example's two iterations so the
`Some(p)` branch would execute, with a tuple, if compilation succeeded.

## The two incompatible requirements

1. literal-types §10, `4.8/known_output.i`: “(run), one case for each hidden
   input of decision 7, each `True` or stopping when run and each giving
   neither E0520 nor E0501.” That row explicitly includes:

   ```i
   fold(xs, None, (acc, x) {
     acc |> {
       Some(p) -> Some((p.0 + 1, x))
       None -> Some((0, x))
     }
   })
   ```

2. literal-types decision 7: “a `Never` at a covariant place becomes `_`”
   and “`Option(Never)` (of `None`) becomes `Option(_)`”. The same decision
   requires this in the expected type of every argument. The callback
   therefore gets `acc: Option(_)` as its known type. The name `p` gets
   the known type `_`. Rule 13, in literal-types §6's replacement §4.8
   and installed spec §4.8, says: “For `v.k`, `v` must have a known type
   and every member must be a tuple with more than k parts.” It requires
   rejection of `p.0` with E0506. Inferred tuple bounds cannot supply a
   known type: spec §4.8's known-type definition uses written facts and
   its matching rules, separately from inference.

Thus the required runnable example and the stated known-type rules cannot
both be implemented unchanged. No later literal parsing or narrowing can
supply the missing known tuple shape at this expression.

The smallest demonstrated repair to the example is
`Some(p: (Int, Int)) -> Some((p.0 + 1, x))`. This preserves the hidden-input
and empty-start regression while supplying the required written shape.
It is a proposed source amendment, not an adopted interpretation. Changing
rule 13 or the known-type matching rules would be a larger language change.

## Measurements

From the lane directory, the reduced check is:

```sh
direnv exec /home/agent/Programming/i \
  target/t1/continued/stage7-work/i-stage7 check \
  --project tests/t1-blockers/none-accumulator --types
```

The saved stage-7 binary, implementing the required whole-literal rule,
checks `tests/t1-blockers/none-accumulator` with exit 1 and exactly:

```text
src/main.i:8:22: error[E0506]: `.k` needs a known type for its value (§4.8 rule 13); write its type, such as `x: (A, B)`
```

The provided baseline `i-1df2fe0` rejects the unannotated example with E0501
at the callback: `(Int, _)` is not a subtype of `Never`, plus W0501 for
`Some(p)`. The stage-7 rule removes that false bound, as required, but
exposes the missing known tuple shape. Merely retaining the baseline's
behavior would also violate §10.

With the written tuple annotation, the saved stage-7 binary runs an entry
that asserts the final result is `Some((1, 2))`, at both `--opt 0` and
`--opt 2`, each with exit 0. This is a substitute program demonstrating a
possible amendment; it does not satisfy the unamended acceptance example.

Artifacts and logs remain in this lane only:

- `target/t1/continued/none-accumulator`: reduced unannotated project.
- `target/t1/continued/none-annotated`: proposed amended project.
- `none-committed-current.log`, `none-minimal-current.log`,
  `none-check-status.log`, `none-baseline.log`,
  `none-annotated-o0.log`, `none-annotated-o2.log`, under the same directory.
- `target/t1/continued/stage7-work/i-stage7`: saved experimental binary.
- `stage7-work/stage7.patch`: tracked stage-7 source work based on cb83db1.
- `stage7-work/new-sources.tar.gz`: seven new source/test files.
- `stage7-work/manifest.json`: exact path inventory and base commit.
- `stage7-work/stage7-goldens.patch`: generated-output audit only; regenerate
  outputs with UPDATE=1 after restoring the implementation, never apply
  this patch as a golden edit.

The source patch and archive were saved before restoring the branch to
stage 6b. They retain the implementation, eight interpretation tests, and
four runtime/error fixtures, including the failing acceptance example.
They are unfinished work, not a verified stage-7 commit. Stage 8, fitted
choices, performance and final soundness review have not started.


## Resolution (2026-11-10)

The coordinator ruled: “docs/spec.md §4.8 rule 13 and decision 7 stand”.
The §10 acceptance example now writes `Some(p: (Int, Int))`. The reduced
unannotated package remains unchanged and is checked by the exact E0506
assertion in `ilang-check/tests/literal_types.rs`. The implementation work
has resumed from its saved patch; the earlier conflict record is historical.
Whether rule 13 may take tuple shape from inference without losing principal
types remains an open language question for a later track, outside T1.
