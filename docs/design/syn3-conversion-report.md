# Syntax three: the conversion of the repository (stage 8, lane/conv)

Status report of the whole-repository conversion to syntax three
(`docs/design/syntax-3.md` §7.2 stage 8, §7.3). It is kept up to date while the
lane runs; the lists it names are the authority.

## How the tree is converted

- `scripts/syn3-convert-all` converts every tracked `.i` and `.is` file with
  `migrate-syntax3 convert`, then the i snippets in Rust and JavaScript strings
  and Markdown fences (`scripts/syn3-convert-embedded`), folds the staged
  syntax-three text of the spec (`scripts/syn3-spec-fold`), and runs the hand
  rewrites (`scripts/syn3-hand-rewrites`).
- A run on a newer tree reads only the `.i`/`.is` files changed since the last
  conversion commit, and only the strings and fences whose text was not in the
  tree of that commit (`SYN3_EMBEDDED_ALL=1` reads all). This settles the texts
  that both syntaxes read to different trees: `T = (A)` is an alias before and
  a labelled type after. Before this rule (fixed in 7405234b) the re-run
  9fd733b4 read hand-written `Box(A) = (A)` snippets again as the old syntax
  and turned them into names `Box(A) = { (A) }`; those were restored.
- The syntax-three parser is the default; the old parser stays behind the
  cargo feature `legacy` of `ilang-syntax`, for the migrators only.

## What is not converted, or converted with a loss

- `scripts/syn3-unconvertible.txt`: files and snippets that neither syntax
  reads (parse-error fixtures of the old syntax). Left as they are.
- `scripts/syn3-lossy.txt`: files the migrator converts with a loss it
  reports. `syn3-convert-all` fails on any other file with such a note. Kinds:
  - `open-record`, `string-label`: open records and label parameters
    (`lib/i/record.i`, the record fixtures); removed by stage 6.
  - `two-row-variables`, `need-not-named`, `row-difference`: need rows that
    the syntax-three `<-` list cannot write.
  - `type-params`: type parameters of a binding whose value is not a function
    are dropped (§2.16 has them only on function literals).
  - `constructor-name-clash`: a constructor named like its type that has
    other constructors; one labelled type per constructor needs a rename by
    hand.
  - `labelled-call`: a labelled call of a function whose parameters the text
    does not show; the stage 4 migrator (`labels`) makes it positional.
  - `empty-match`: `x |> {}` becomes `core.never(x)` (§2.13); the module must
    import `i/core`.
- `tests/spec/syn3-pending.txt`: spec cases skipped until stage 6 (open
  records) lands.
- `crates/ilang-check/tests/syn3-pending.txt`: checker tests that use a form a
  later stage removes, named with that stage; each returns early and says so.
- Spec fences that need forms of later stages (shape groups, stage 9;
  selection, stage 11; `..env`, E6) keep the staged `i future-SYN3` form with an
  inventory entry until that stage lands.

## Behaviour changes

1. **A function literal's `-> R` no longer closes its needs.** Syntax three
   writes needs only after `<-`, and the whole need is inferred (§2.10.1,
   "Type: the whole need, inferred"). So `f = () -> Int { s |> ("a" | "c") { 1 } }`
   with `s = "b"`, which was E0501 (`Assert` not allowed: the old `-> Int`
   with no `!` allowed no needs), now compiles: the one-branch group is fitted
   and "when it may fail it reports to `Assert`" (spec §6.12), so `f` needs
   `Assert`.
   The check that remains is the warning: no value of the known type `"b"`
   matches `"a" | "c"`, and "a branch that can never be used gives a compile
   warning" (spec §6.12; syntax-3 §2.12, D4). W0505 was given only for
   constructor patterns (at fitting); it is now also given for every fitted
   pattern whose failure is settled (one-branch matches and bindings, literal
   patterns included) when no value of the known type matches it
   (`Checker::warn_never_matches`, `types/infer.rs`). Test:
   `literal_types::fitted_choices_keep_the_deferred_binding_error`.
   In short: an error became a warning plus an `Assert` need.
2. **Still to come: sums are unions, not families of constructors** (§2.8).
   The lowering of this stage still reads `Event(A) = Start(A) | Stop(A)` with
   its members just before it as one type with two constructors (today's tree),
   so facts "across every constructor of the family" (literal-types §5.3) are
   unchanged, and a member of such a union written as a type elsewhere
   (`f = (x: Start(Int))`) is not defined (E0202). Lane/fa's final tree makes
   each member its own labelled type. Test that will change then:
   `types::pattern::shared_place_tests`.
3. **`()` as a value is the empty data** of type `()` (§7.1 row 7): `f = (()) { 1 }`
   is called as `f(())` (fixture `tests/spec/7/syn3_empty_data_argument.i`).
   The Lisp surface reads `()` as the same empty tuple.
4. **Lowercase type parameters** (`Codec(d, e) = …`): the old compiler read
   the head and rejected it (E0232); syntax three rejects it in the parser
   (E0101, §3.2), so the rest of such a file is no longer checked. The
   converter notes them (`lowercase-type-param`, a loss), and the five review
   cases that are error fixtures record E0101.
5. **`fail: Fail(E)` in a function head** is the function's error part, as
   `Fail(E)` in a need of a function type: the head has no service `Fail`
   (`Cx::params_of`, `ret`; patch from lane/cG2).

## Defects found and fixed during the conversion

- The lowering read `T(N) = (N)` followed by `Alias(N) = T(N)` as the old
  one-constructor sum `Alias(N) { T(N) }` (the form the converter writes for
  `TimeError { DivisionByZero }`), so `T` stopped being a type (E0202 on later
  uses). A one-member right side is now a one-constructor type only when its
  member is not written as a type elsewhere (`syn3/lower.rs`; test
  `syn3_lower::one_member_alias_keeps_a_type_written_as_a_type`).
- The legacy printer refused the empty tuple that `()` now lowers to.
