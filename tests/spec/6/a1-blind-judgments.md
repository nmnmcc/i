# Frozen blind A1 syntax cases

Language source: `/home/agent/Programming/i/.rd/a/docs/design/syntax.md`, read fully. No implementation, existing tests, a1-front-end.md, git diff, log, other worktree, compiler, or test run consulted. All files were authored before any implementation result. These files and expectations are now frozen.

The `spec:` headers refer to the replacement sections in syntax.md. Each ordinary single file is a package spec module `spec/<filename stem>`. No std dependency is needed. Runtime entry points are exported except the intentional negative entry test.

## Decisive expectations and exact source sentences

### `tests/spec/6/a1_opening.i` — `pass`

- Exact syntax.md source (`opening_body`): A `{` that begins an expression is a clause block or a block by its first statement, but the body of a function is always a block (§6.1).
- Exact syntax.md source (`opening_annotation`): `x: T` in a clause pattern is a type annotation when the clause block has one clause, no guard, and no choices, and a test when not (§7.1). So with `v: Int | String`, `v |> { x: Int -> x }` is a compile error, but `v |> { x: Int -> x  _ -> 0 }` is correct.

### `tests/spec/7/a1_one_clause_annotation.i` — `compile-error ?`

- Exact syntax.md source (`opening_annotation`): `x: T` in a clause pattern is a type annotation when the clause block has one clause, no guard, and no choices, and a test when not (§7.1). So with `v: Int | String`, `v |> { x: Int -> x }` is a compile error, but `v |> { x: Int -> x  _ -> 0 }` is correct.

### `tests/spec/1.7/a1_line_breaks.i` — `pass`

- Exact syntax.md source (`line_comma`): Directly inside `( )`, `[ ]`, or `<< >>`, a break is the same as a `,`. A `,` at the end of a line may be left out.
- Exact syntax.md source (`line_negative`): `a` and then `- b` on the next line are two statements. To write `a - b` on two lines, put `-` at the end of the first line. In the same way, `! N` of a function type may not begin a line.
- Exact syntax.md source (`line_continue`): A line that ends in an operator, `,`, `=`, `:`, `->`, `(`, `[`, `{`, or `<<` goes on in the next line. A line that begins with `|>`, `.`, `|`, `?`, `->`, or a binary operator other than `-` and `-.` goes on from the line before.
- Exact syntax.md source (`line_parenthesis`): When `(` is on the same line as the expression before it, it is a call (§6.6). When `(` is the first thing on a line, it begins a new statement or part.

### `tests/spec/1.7/a1_same_line_tuple.i` — `compile-error ?`

- Exact syntax.md source (`line_bad`): But a clause or a statement whose pattern starts with `(`, `-`, or `-.` begins a new line, because the expression before it on the same line goes on with it. In `p |> { (0, y) -> y  (x, _) -> x }`, `y(x, _)` is read as a call, and in `n |> { 0 -> a  -1 -> b }`, `a -1` is read as `a - 1`; in each case, what is left is a compile error.

### `tests/spec/1.7/a1_same_line_negative.i` — `compile-error ?`

- Exact syntax.md source (`line_bad`): But a clause or a statement whose pattern starts with `(`, `-`, or `-.` begins a new line, because the expression before it on the same line goes on with it. In `p |> { (0, y) -> y  (x, _) -> x }`, `y(x, _)` is read as a call, and in `n |> { 0 -> a  -1 -> b }`, `a -1` is read as `a - 1`; in each case, what is left is a compile error.

### `tests/spec/2.2/a1_headers_tuple.i` — `pass`

- Exact syntax.md source (`headers`): When an item in it has a module path (§2.3), which always has a `/`, the group is the **imports**.
- Exact syntax.md source (`tuple_binding`): A group in `( )` that is followed by `=` is a binding with a tuple pattern, as in `(even, odd) = …`.
- Exact syntax.md source (`imports_grammar`): ImportItem = [ name "=" ] [ "(" ImportName { "," ImportName } [ "," ] ")" "=" ] ModulePath .

### `tests/spec/2.2/a1_empty_header.i` — `compile-error ?`

- Exact syntax.md source (`empty_header`): Neither may be empty: a file with nothing to import or export leaves the group out.

### `tests/spec/2.2/a1_header_order.i` — `compile-error ?`

- Exact syntax.md source (`header_order`): A file has at most one imports group and at most one exports group. They come first, the imports before the exports.

### `tests/spec/2.2/a1_macro_attribute_line.i` — `compile-error ?`

- Exact syntax.md source (`macro_attribute`): Any other attribute, such as `@m`, `@m(args)`, or `@a.m(args)`, is a macro before an item. It is kept for macros, and it is a compile error whose message says so.
- Exact syntax.md source (`attribute_next_line`): So when an attribute has no arguments and the item after it starts with `(`, as the binding `(even, odd) = …` does, the item begins on the next line:

### `tests/spec/2.2/a1_macro_attribute_same_line.i` — `compile-error ?`

- Exact syntax.md source (`attribute_same_line`): On one line, `@m (even, odd) = …` is the attribute `@m(even, odd)` followed by `= …`, which is a compile error.

### `tests/spec/2.2/a1_builtin_attribute_call.i` — `compile-error ?`

- Exact syntax.md source (`builtin_args`): Its arguments must be string literals, given by position, with no labels, and an attribute that takes no arguments is written with no `( )`, as in `@main`. Any other argument, such as `@external("env" <> "", "f")`, `@external(module: "env", "f")`, or `@main()`, is a compile error.

### `tests/spec/2.2/a1_macro_item.i` — `compile-error ?`

- Exact syntax.md source (`macro_item`): A `MacroItem` is kept for macros. It is a compile error, and the message says that a call at the top level is kept for macros.

### `tests/spec/2.2/a1_macro_declaration.i` — `compile-error ?`

- Exact syntax.md source (`macro_declaration`): A declaration with no `@external` is kept for macros, where it goes with a binding of the same name. It is a compile error, and the message says that a declaration with no `@external` is kept for macros.

### `tests/spec/3/a1_macro_type_call.i` — `compile-error ?`

- Exact syntax.md source (`type_call`): A lowercase `name`, or `a.m` with a lowercase `m`, whatever `a` is, followed by `(` is a `TypeCall`. It is kept for macros, and is a compile error.

### `tests/spec/4.7/a1_declared_variables.i` — `pass`

- Exact syntax.md source (`declare_variables`): A binding whose pattern is one name, with or without a type, may declare type variables after the name (§4.7): `map(A, B) = …`, `id(A): (A) -> A = (x) { x }`.
- Exact syntax.md source (`scope_variables`): A type variable declared by a binding may be used in the type on its pattern, as in `id(A): (A) -> A = …`, and in every type written in its value, also in the parameters, `->` parts, and clause patterns there, and in inner bindings.
- Exact syntax.md source (`inner_hiding`): An inner binding may declare a type variable with the same name; inside it, the name means its own variable.
- Exact syntax.md source (`generic_head`): The type parameters are the type variables of the definition: `Option(A) { Some(A) None }`. Every type parameter must be used on the right, and every type variable on the right must be a parameter.

### `tests/spec/4.7/a1_duplicate_variables.i` — `compile-error ?`

- Exact syntax.md source (`duplicate_variables`): The type variables declared in one place must have different names. So `pair(A, A) = …` is a compile error.

### `tests/spec/4.7/a1_undeclared_variable.i` — `compile-error ?`

- Exact syntax.md source (`undeclared_variable`): An `UpName` in a type that is neither a type nor a declared type variable is a compile error. The message says how to declare it.

### `tests/spec/4.7/a1_variable_hides_type.i` — `compile-error ?`

- Exact syntax.md source (`hides_type`): A declared type variable may not have the name of a type that may be written with no module name in the module (a type of the module, an imported `UpName`, or a built-in type), because then a type would mean two things.

### `tests/spec/3/a1_variable_arguments.i` — `compile-error ?`

- Exact syntax.md source (`variable_args`): A type variable takes no arguments: `A(Int)`, where `A` is a declared type variable, is a compile error.

### `tests/spec/4.3/a1_custom_type_head.i` — `compile-error ?`

- Exact syntax.md source (`head_type_name`): When every part is such an `UpName`, but one of them is a type that may be written with no module name in the module, as in `Box(Int) { B(Int) }` or `Pair(User) { Pair(User, User) }` with a type `User`, the message says that a type parameter may not have the name of a type.

### `tests/spec/4.3/a1_alias_pattern.i` — `compile-error ?`

- Exact syntax.md source (`alias_pattern`): An alias whose head is not a list of type parameters, as in `Schema(User, _) = user`, or `Schema(User) = user` with a type `User`, is kept for a later use, alias patterns, and is a compile error whose message says so.

### `tests/spec/6/a1_fitting_expected_subject.i` — `pass`

- Exact syntax.md source (`one_clause_fitting`): A clause block with one clause and no guard is the same as a function with one parameter: `{ p -> e }` is `(p) { e }`. Its pattern is fitted to the type of the parameter (§4.8 rule 4), and it is not checked for coverage.
- Exact syntax.md source (`known_subject`): In a match `e |> { … }`, every clause pattern is a pattern fitted to a value with the known type of `e` (below), also when the clause block has one clause and no guard. So with `opt: Option(User)`, `opt |> { Some(u) -> u.name }` is correct, as `Some(u) = opt` followed by `u.name` is.
- Exact syntax.md source (`expected_subject`): In a match `s |> { p -> e }` whose clause block has one clause, no guard, and no choices, when `p` has `x: T` at its top, `s` has the expected type `T`, as the value of a binding `x: T = s` does. So `(u) { u.name } |> { f: (User) -> String -> f(bob) }` is correct, as `f: (User) -> String = (u) { u.name }` followed by `f(bob)` is.
- Exact syntax.md source (`assert_effect`): When `p` may fail, it reports to `Assert`, as a parameter does (§6.11), and the first position of the stop message (§11.7) is the position of `p`, as for a parameter.

### `tests/spec/6/a1_assert_required.i` — `compile-error ?`

- Exact syntax.md source (`assert_effect`): When `p` may fail, it reports to `Assert`, as a parameter does (§6.11), and the first position of the stop message (§11.7) is the position of `p`, as for a parameter.

### `tests/spec/6/a1_expected_argument.i` — `pass`

- Exact syntax.md source (`expected_argument`): When the parameter has a known type `B`, from a written type or from an expected type, every clause pattern is a pattern fitted to a value with the known type `B`, as in a match (the bullet on matches, before **Names in patterns**, below), and the narrowing by earlier clauses of §4.8 rule 12 is used in the same way. So with `opts: List(Option(User))`, `list.map(opts, { Some(u) -> u.name  None -> "" })` is correct, as `list.map(opts, (o) { o |> { Some(u) -> u.name  None -> "" } })` is.

### `tests/spec/6/a1_brace_decisions.i` — `pass`

- Exact syntax.md source (`brace1`): When the statement starts with `name ( UpName { , UpName } )` and then `=` or `:`, it is a binding with type parameters (last bullet), and not a clause, though `f(A)` could also be read as a constructor pattern (§7.1).
- Exact syntax.md source (`brace2`): Then the statement is the binding of the pattern `n1 = … = nk` to the function that starts at `( … )`.
- Exact syntax.md source (`brace3`): When the statement starts with a pattern and then `=`, it is a binding (last bullet), and not a clause.
- Exact syntax.md source (`brace4`): When the statement starts with `name :`, and is not a pattern `name: Type` followed by `=`, `->`, `?`, or `|`, the `{` is kept for records (below), and it is a compile error.
- Exact syntax.md source (`brace5`): This test is only made for a statement that the tests before it do not decide, so a `->` in a later statement on the same line, as in `{ k = 2  g = (x) -> Int { x * k }  g(1) }`, does not count.
- Exact syntax.md source (`brace6`): When none of these holds, the statement is an expression, and not a clause.
- Exact syntax.md source (`brace_examples`): So these are blocks:

### `tests/spec/6/a1_record_kept.i` — `compile-error ?`

- Exact syntax.md source (`brace4`): When the statement starts with `name :`, and is not a pattern `name: Type` followed by `=`, `->`, `?`, or `|`, the `{` is kept for records (below), and it is a compile error.

### `tests/spec/6/a1_wrong_pattern.i` — `compile-error ?`

- Exact syntax.md source (`wrong_pattern`): So `x |> { -y -> 1 }` and `b |> { <<x: Int>> -> 1 }` give an error in the pattern, and not a block with a stray `->`.

### `tests/spec/6/a1_pipe_groups.i` — `pass`

- Exact syntax.md source (`pipe_precedence`): `|>` is weaker than `+` and stronger than the operators of levels 1 to 5: `||`, `&&`, `==`, `!=`, comparisons, and `<>`.
- Exact syntax.md source (`pipe_group`): So a match that is an operand of such an operator or of `-` or `-.`, or that has `.f`, `.k`, or a call after it, is put in a group: `{ n |> { 0 -> 1  _ -> 2 } } * 10`, `1 + { n |> { … } }`, `-{ n |> { … } }`, `{ opt |> { … } }.name`.

### `tests/spec/6/a1_unused_clause.i` — `warning ?`

- Exact syntax.md source (`unused_clause`): Where this document speaks of a function expression, a clause block counts as one, unless the text says otherwise. So a clause block whose value is not used gives a compile warning (§6.10), and each time a clause block is worked out, it makes a new value, which is the same only as itself (§11.4).

### `tests/spec/6/a1_labels_and_catchall.i` — `pass`

- Exact syntax.md source (`labels`): The parameter of a clause block with one clause, no guard, and no choices has the label of its pattern, as in `(p) { e }` (§6.11): `{ n -> n }` has the label `n`, and `{ Some(u) -> u }` has none. The parameter of any other clause block has no label, also when its one clause has choices, as in `{ v = Ok(_) | v = Error(_) -> v }`.
- Exact syntax.md source (`catchall`): If some clause without a guard has a catch-all at that place or at a place above it, then `B <: P | r - P`, with a new type variable `r`: a value that no pattern takes may still come there.
- Exact syntax.md source (`guard_function`): In the same way, a `?` after a function type is part of the type, so a function type before a guard is put in `{ }`: `f: { (Int) -> Int } ? ok -> …`.

### `tests/spec/6/a1_no_label.i` — `compile-error ?`

- Exact syntax.md source (`labels`): The parameter of a clause block with one clause, no guard, and no choices has the label of its pattern, as in `(p) { e }` (§6.11): `{ n -> n }` has the label `n`, and `{ Some(u) -> u }` has none. The parameter of any other clause block has no label, also when its one clause has choices, as in `{ v = Ok(_) | v = Error(_) -> v }`.

### `tests/spec/6/a1_empty_never.i` — `pass`

- Exact syntax.md source (`empty_never`): `{}` is a clause block with no clauses. Its parameter has the type `Never` (§4.8 rule 11), so `x |> {}` is correct only when `x` has no values.

### `tests/spec/6/a1_empty_inhabited.i` — `compile-error ?`

- Exact syntax.md source (`empty_never`): `{}` is a clause block with no clauses. Its parameter has the type `Never` (§4.8 rule 11), so `x |> {}` is correct only when `x` has no values.

### `tests/spec/7/a1_coverage_guard.i` — `compile-error ?`

- Exact syntax.md source (`coverage`): Any other clause block, with no clauses, with more than one clause, or with a guard, is typed by §4.8 rule 11, and its clauses must cover every value of the type of its parameter (§7.4). If not, it is a compile error.
- Exact syntax.md source (`guard_coverage`): The clauses of a clause block whose parameter has the type `A` **cover** `A` when, for every value of `A`, some clause without a guard has a pattern that matches it. A clause with a guard is not counted.

### `tests/spec/7/a1_no_subject_constructor.i` — `compile-error ?`

- Exact syntax.md source (`no_subject_constructor`): So `opt |> { Some(x) -> x }` is the same as the binding `Some(x) = opt` followed by `x`, and it gives only what that binding gives: `x` gets its known type in the same way (§4.8), and `opt` is not known to be made by `Some` (§7.5).

### `tests/spec/10/a1_entry_exports.i` — `compile-error ?`

- Exact syntax.md source (`entry_exports`): "They must be in the `export` list of their module." becomes "They must be in the exports of their module."

### `tests/spec/11/a1_identity_recursion_tail.i` — `pass`

- Exact syntax.md source (`direct_recursion`): A clause block is a direct value (§6.10), as every function is. Where §2.5, §5, and §6.10 speak of the function bodies in a value, the guards and clause expressions of a clause block count as function bodies. So `loop = { 0 -> Done  n -> loop(n - 1) }` is correct: `loop` is not among its own needs, and as a local binding it may be used in its clause expressions.
- Exact syntax.md source (`identity`): So a clause block whose value is not used gives a compile warning (§6.10), and each time a clause block is worked out, it makes a new value, which is the same only as itself (§11.4).
- Exact syntax.md source (`tail`): A call in tail position uses no more stack.
- Exact syntax.md source (`match_tail`): A match adds no call frame: the clauses run in the place of the match, so a call in a clause expression of a match in tail position is in tail position (§11.6), and the match is not one of the calls in a stop message (§11.7).

### `pending/tests/spec/6/one_clause_choices.i` — `pass` **PENDING E0235 until T1; intended semantic expectation remains success.**

- Exact syntax.md source (`one_clause_choices`): When that one clause has choices, as in `{ Ok(v) | Error(v) -> v }`, the same rule holds, and the choices are fitted together (§4.8).

## Decisions and uncertainties

- Assert is checked through an explicit `! Assert` function type and a contrasting function type without that requirement. The source says the failing pattern reports to Assert but does not give the exact first stop-message text; no invented stop string or location oracle was added.
- Brace tests 1–3 and 6 have fully defined positive examples. Test 4 has both a valid annotated binding and a standalone records rejection. Test 5 has the fully defined later-function example (which must not decide the first brace), plus a standalone malformed `-y` pattern rejection. The examples avoid the temporary excluded lone clause block whose first pattern is parentheses and expression starts with a function.
- The built-in assertion and main harness, core.not, field access, list construction and equality follow the syntax shown or named in syntax.md and the coordinator-provided harness constraints. No implementation was used to refine these ordinary operations.
- The generic catch-all is deliberately called with String: a previous literal Int clause must not close the parameter type to Int.
- Positive tests are allowed to include harmless ordinary unused-binding warnings; only a dedicated warning file requires a warning. The specification gives no stable diagnostic code/text for most errors, so error headers use `compile-error ?`.
- The million-step top-level, local, and match recursion cases are finite and demand stack-safe tail calls; they do not assert a particular performance limit.
- No keywords-as-names, lowercase-type-error cases, or lowercase/mixed custom type heads are tested under the temporary A1 exclusions.
- Pending one_clause_choices is outside the active `tests/spec` subtree. Its pass header is unchanged by the temporary E0235 limitation; do not copy it into active A1 tests before T1.

Frozen: do not modify these authored files or semantic expectations in response to implementation results. No commits made by the writer.

## Supplemental exact source sentences

- The positive brace alias-pattern examples: A `{` that begins an expression begins a **clause block** when it is `{}`, or when its first statement is a clause (next bullet). When not, it begins a block.
- Generic aliases in declared_variables: `T = Type` is a type alias. `T` is the same as the type on the right. An alias may not use itself, directly or through other aliases.
- Valid annotated binding in brace_decisions: A `{` that begins an expression and is followed by `name :` begins a clause block or a block when the pattern `name: Type` is followed by `=`, `->`, `?`, or `|`, as in `{ x: Int -> x }` and `{ x: Int = f()  x }`.
- Negative Float clause on a new line in line_breaks: But a clause or a statement whose pattern starts with `(`, `-`, or `-.` begins a new line, because the expression before it on the same line goes on with it.
- Top-level and local tail-call positions in identity_recursion_tail: Tail positions are: the last expression of the body of a function expression (§6.8); the expression of a clause of a clause block that is called as a function, and not in a match, because that expression is the last thing the function works out; the expression of a clause of a match in tail position; the last expression of a block in tail position; and the right side of `&&` or `||` in tail position.

## S4 judgment

The writer froze 38 cases before seeing implementation results. The frozen record
above describes those original expectations. The coordinator's S4 ruling accepts
corrections for two test bugs against the spec. The merged set has 37 active
cases and one pending T1 case. The 25 unknown diagnostic codes were filled only
after checking the messages against the source sentences above.

- `10/a1_entry_exports.i` originally expected a compile error. Its header now
  expects success; the program is unchanged. Spec §10.2 says "The program entry:
  the function marked `@main` in the package that is compiled." Only the
  host-export bullet says "They must be in the exports of their module."
  The original writer applied that restriction to the wrong bullet. A1 §6.2
  is corrected too. A private `@main` remains valid.
- `4.7/a1_declared_variables.i` keeps its success expectation, with `! Assert`
  added to the explicit return part of `outer`. Spec §6.8 says "When the `->`
  part is written, it is the type of the function after its parameters, so a
  missing `?` or `!` in it means `Never` (§3)". Spec §9.6 gives `assert` the
  environment `Assert`. The original E0501 was correct; the case now isolates
  the intended declared-variable scope rule.

The original frozen files remain in the lane's audit artifacts. These two
source-based corrections are the only changes to program bytes or semantic
expectations after the freeze; no compiler behavior was changed to fit them.

`6/a1_one_clause_choices.i.pending` keeps its frozen success expectation.
Syntax.md §6.12 requires that result, but D7 explicitly defers implementation to
T1 and requires E0235 in A1. This is a known, accepted gap, not satisfaction of
the pending success expectation.

The writer did not invent a stop-message oracle from syntax.md alone. The S2
runtime test separately checks the prescribed position of a failing pattern.
