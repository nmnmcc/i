# Macro execution charges

Each macro invocation has a fresh budget. The compiler selects 100,000,000
work units, 2^28 allocation words and 10,000 active frame units; a package
cannot change them. A word is eight bytes. The expansion nesting limit is
256 and is enforced by the expansion driver. The first exhausted limit wins
and the compiler reports that limit at the macro call. Exhaustion is not an
i error and cannot be caught by the macro.

The table defines accounting before Wasm optimization. Charges are inserted
in the canonical lowered program and its runtime, rather than counted from
Wasm instructions or Wasmtime fuel. The macro program always targets the
compiler's i host, independently of the target of the program being built.

| Action | Work units | Allocation words | Frame units |
|---|---:|---:|---:|
| Execute one lowered i/IR statement | 1 | 0 | 0 |
| Enter a generated or runtime function | 1 | 0 | 0 |
| One iteration of a runtime loop | 1 | 0 | 0 |
| Bulk copy or fill | number of elements | 0 | 0 |
| Construct a structure | number of fields | 4 plus one per field | 0 |
| Construct an array | number of elements | 4 plus the element bytes rounded up to words | 0 |
| Grow linear memory | 65,536 per Wasm page | 8,192 per Wasm page | 0 |
| Enter an i source function | 0 | 0 | 1 plus parameters, local/captured names and the maximum pending expression values |
| Enter a synthesized callable wrapper | 0 | 0 | 1 plus parameters |
| Compiler Syntax operation | 1 plus nodes and bytes traversed | wrapper plus output and traversal words | traversal depth |
| Copy a boundary buffer | bytes copied | 4 plus bytes rounded up to words | 0 |
| Expand Syntax into a scoped AST | 1 plus one per node and child | 1 plus 64 per node and 8 per child | 0 |
| Render a source-name or text atom for scoped AST | 24 times its UTF-8 bytes plus 16 | 4 plus that bound rounded up to words | 0 |
| Render an integer atom for scoped AST | square of its 32-bit limb count plus its bit count plus 1 | 4 plus its bit count plus 1 bytes, rounded up to words | 0 |
| Render a float atom for scoped AST | 32 | 8 | 0 |
| Look up a host identity among `n` live or weak entries | `16 × (1 + bit_length(n + 1))` | 0 | 0 |
| Install a host identity | the lookup cost again | `64 × (1 + bit_length(n + 1))` | 0 |
| Sweep weak host identities | number of entries | 0 | 0 |

Array element widths are taken from the generated type: i8 is one byte, i16
two, i32/f32 four and references/i64/f64 eight. Fixed runtime globals and
constant closures are charged before Wasm instantiation from a compiler-made
`ilang.macro.budget` section (one work unit per allocation in addition to its
elements); this is necessary because Wasm constant
expressions cannot call the accounting hooks. Each dynamic allocation is
charged before it happens. Length operands are evaluated exactly once.

The count is cumulative even if a value is collected or shared later. A
runtime function that works on a value charges its loop steps and the size of
bulk copies, so multiplying large integers or copying a large string cannot
become a single unit. Compiler Syntax operations charge their source tree and
worst-case escaped output before traversing or building the result. The host
and guest use the same invocation budget; cloning the handle does not reset it.
Host identities use a per-instance balanced map, so a lookup and an insertion
have a deterministic size-based charge before comparison, allocation or sweep.

Tail calls leave the caller's frame before the callee enters. Error handlers
restore the saved frame depth, and resumed continuations restore their own
source frame. A genuine machine stack or heap exhaustion remains a compiler
resource failure rather than a macro answer or a cached SyntaxError.

Pending values are counted from source evaluation order, not Wasm locals:
the callee and earlier arguments remain pending while a later argument is
evaluated; tuple, list and record elements follow the same rule. Binary
expressions keep their left operand while evaluating the right. A branch
keeps its subject while testing its clauses. The maximum of the function's
source expressions and patterns contributes to each of its frame units;
syntax of a nested function belongs to that nested function. This accounts
for a large call whose many evaluated arguments remain live across a recursive
call, without charging a different amount when an optimizer changes locals.

Reference: Lean's `src/Lean/CoreM.lean` and `src/Lean/Util/Heartbeats.lean`
keep deterministic heartbeats separate from recursion depth. i additionally
accounts for size-dependent runtime work and allocation, so a single call to
a primitive cannot hide an unbounded copy or integer operation.
