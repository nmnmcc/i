# Consistency audit: decision table

State: 2026-11-21, against `docs/spec.md` at `2c2923f8a`. This document is the
decision record for the consistency audit: one row per place where the standard
and the compiler, or two parts of the standard, disagree. It does not change the
standard or the compiler.

**Every line and PR uses these IDs.** A PR says `fixes B30`, not a paraphrase.
IDs are permanent: a row is closed by setting its status, never by renumbering.
An implemented row keeps its ID and a `fixed (<commit>)` status.

### How to read a row

- **ID**: `A…`/`B…` are the spec-versus-implementation audit (contradictions and
  deviations). `KA…`–`KH…` are the spelling/consistency audit, by topic; the
  letter after `K` is the topic and the number is the row. A row is cited by ID
  alone.
- **Class**: `contradiction` (two standard sentences), `deviation`
  (implementation differs), `real inconsistency`, `justified exception`,
  `conflict` (standard and implementation already disagree; a decision is
  needed), `diagnostic`.
- **Severity**: `S1` wrong programs / crash / hang / valid code rejected;
  `S2` wrong acceptance, silent loss, or a rule that cannot be used; `S3`
  misleading diagnostics; `S4` wording or a justified exception.
- **Decision**: the chosen unification, or `needs owner` with numbered options.
  A row never ends at "to discuss".
- **Side**: `impl`, `spec`, `both`, `verify`, or `none`.
- **Migration count**: call sites/files/tests to touch, or `unmeasured`.
- **Owner line** and **depends on** make the schedule explicit.

Status: implementation rows are tracked as `open`, `in progress` or `fixed` in
[`spec-known-gaps.md`](spec-known-gaps.md). A rows carried by PR #56 are
`in progress`; the held A4/A7/A8 are `needs owner`. K rows are `open` unless
the decision is a justified exception.

## 1. Contradictions inside the standard (A)

PR #56 already carries A1, A2, A3, A5, A6, A9, A10 and KB3. A4, A7 and A8 are held for the owner; A4's proposal there is option 1.

| ID | Sev | Class | Standard location and sentence | What conflicts | Evidence | Decision | Side | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| A1 | S3 | contradiction | §9.3 l.1453 and §9.4 l.1475 say a stale boundary "stops the program with the message `boundary not active`"; §11.7 l.2621 says the reason is `control: boundary is not active`; the spec's own test `tests/spec/9/boundary_inactive.i:2` expects the latter | Two different required stop messages | Runtime emits `control: boundary is not active` (`crates/ilang-wasm/src/runtime/stops.rs:14`, probe G/e3) | 统一为 `control: boundary is not active`；改 §9.3、§9.4，§11.7 与测试已如此。 | spec | 0 | docs | — |
| A2 | S1 | contradiction | §9.9 l.2045 "v1 has one worker throughout"; §11.9 l.2672 "A program may run on one or more **workers**" | Whether the entry run is single-worker | `crates/ilang-host/src/workers.rs:39` defaults to `available_parallelism()`; `I_WORKERS=1` changes observable order (H2/g: with default workers the child starts *after* the body's `yield`, and its final is lost) | 删“v1 has one worker throughout”；§9.9 写明 turn/FIFO 顺序只在单 worker 或 `task.test` 下确定。 | spec | 0 | docs | — |
| A3 | S2 | contradiction | §2.6 l.415 "These keys have a meaning; other root keys are not used" (lists only `name`, dependencies, `[wasm]`); §10.2 l.2525 "With no configured collection, no tests run" | `[test] collection` is a meaningful key but is declared unused | `i new` writes `[test] collection`; `[entries]`/`wasm.synchronous` are also absent from §2.6 | 把 `[test]`、`[entries]`、`wasm.synchronous` 写进 §2.6 的键表。 | spec | 0 | docs | — |
| A4 | S2 | contradiction | §9.5 l.1527 `ensure`'s type is `(() <- N -> A, () <- N -> ()) <- N -> A`, so the final may need `Fail`; l.1553 "final functions … may not raise errors" | Type permits what the prose forbids | A final that calls `fail.raise` compiles and its error reaches the entry as `error: "in final"` (G/f1) | needs owner。PR #56 提案 = 选项 1：保留类型，放宽散文为“release/final 可经其捕获的行里的 `Fail` 报错，但不得增加行外 need”。选项 2：`ensure` 的 final 改 `Fail(Never)`，保留禁令。 | spec | 0 | docs+check | A8 |
| A5 | S3 | contradiction | §2.6.1 l.463 lists platform services as host services + `Assert` + `Fail` + `Root`/`Owner`/`Await`/`Runner`/`Scheduler`; §10.2 l.2528 adds `Scope` and `Target` and special-cases `Fail` for exports | The entry platform list is stated twice, differently | Source only (no probe needed); §9.5 l.1522 also relies on the §10.2 `Scope` | 以 §10.2 的完整清单为准，§2.6.1 引用它而不是重复枚举。 | spec | 0 | docs | — |
| A6 | S2 | contradiction | §10.2 l.2532 (async iff a host service declared `"async"` **or `Runner`**) vs l.2536 (async iff a request-mode host service, **`Runner` or `Scheduler`**) | Two definitions of "async entry"; `Scheduler` differs | Source only | 保留 v3 段（含 `Scheduler`）为准，删前一段或标为旧协议。 | spec | 0 | docs | — |
| A7 | S2 | contradiction | §4.1 l.590 "All function types are one type constructor"; §4.4 l.736 "no two members of a type have the same type constructor"; §3 l.572 "two members may not have one shape"; §4.3 l.680 every sum member must have a different type constructor | Same-constructor members are merged by §4.4 first, so §3's and §4.3's distinctness rules can never fire | `{ (Int) \| (String) }` becomes `(Int \| String)` and `Wrap(Int) \| Wrap(String)` is accepted (C/v5, C/d19) | 待拍板（选项 1：函数仍为一个构造子，重载另立“重载组”构造子；选项 2：删“成员构造子唯一”，改写 §3/§4.3 的不可触发句）。与 T12 一起定。 | spec | 0 | docs+check | T12 |
| A8 | S1 | contradiction | §4.7 l.850 `id = <A>(x : A) -> A { 1 }` "is correct"; §4.8 l.881 "**Every value that an expression can have at run time is in its known type**" | An `A`-typed result can hold `1`, so the known type `A` can be wrong; a later selection can trap | Documented as T8/F2 in `docs/design/spec-known-gaps.md`; not reproduced in this binary (see D2) | 待拍板（选项 1：写类型参数作刚性，拒掉今天能过的程序；选项 2：不从可放宽参数派生 known type；见 T8/F2）。 | impl | 0 | check | T8/F2 |
| A9 | S4 | contradiction | §11.7 l.2632 says positions are `module path:line:column` but the same sentence's example is `app/users : 12 : 5` | Spacing in the documented format | Runtime prints `s4/main:1:35` (I/s4) | 把示例改成 `app/users:12:5`，与格式一致。 | spec | 0 | docs | — |
| A10 | S3 | contradiction | §4.7 l.850 "Inference may make it more exact: `id = <A>(x : A) -> A { 1 }` is correct" vs l.843 "A type variable in an annotation may become a more exact type" | The written `A` receives an extra *lower* bound, so `f("s")` has type `"s" \| 1` — wider, not more exact; `s : String = f("s")` is rejected although the call returns `1` | D/relax2: check 0, run prints `1`; D/relax: E0501 | 改写为“写类型参数是普通推断变量，可能得到双向额外约束；`<A>(A) -> A` 不保证参数性”。 | spec | 0 | docs | — |

## 2. Implementation deviates from the standard (B)

| ID | Sev | Class | Standard location and sentence | Actual | Evidence | Decision | Side | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| B1 | S1 | deviation | §9.9 l.1809 + l.1947: cancellation waits for finals, but an idle run with no waitable work must "stop with panic("task: deadlock")" | With ≥2 workers (`I_WORKERS=2` or the default), a cancelled task whose final blocks on `sync(never())` makes the run **hang forever**; one worker stops correctly | H2/k: `timeout 300 i run` → rc=124 (3x); `I_WORKERS=1` → `panic: task: deadlock`, rc=1; plain `sync(choose([]))` still deadlocks correctly | 改运行时：取消期间的 quiescence 要计入正在收尾的 scope，并保留 deadlock 检测。 | impl | unmeasured | runtime | — |
| B1a | S1 | deviation | §4.7 l.841 "For every binding, the compiler must find the principal type … where it cannot, it must report a compile error" | For `both = <A>(x : { A \| List(A) }) -> List(A) { [] }`, `both([1])` has two typings that are not instances of each other (`A ⊇ Int`, `A ⊇ List(Int)`). The compiler silently picks `A = Int`: `r : List(Int) = both([1])` is accepted, `r : List(List(Int)) = both([1])` is E0501, and no E0504 is reported | D/amb6 → exit 0; D/amb7 → `error[E0501]: 1 is not a subtype of List(Int)`; the E0518/E0504 check only covers unions of ≥2 bare variables (D/amb1–3) | 扩展 E0518/E0504：`A \| C(A)` 这类同变量出现也必须报歧义。 | impl | 0（新增回归） | check | — |
| B2 | S1 | deviation | §4.8 l.889 "When `N` has a rigid row variable, a key that it may carry and that `N` does not state is a compile error" | Accepted silently by `check` and `run` | G/s9; my re-run: `<N>(body : () <- N -> Int) -> Int { body() <- (Clock(now = () { 1 })) }` → exit 0 | 在 rigid row 变量下，provide 未声明 key 时报 §9.2 的编译错误。 | impl | 0（新增回归） | check | — |
| B3 | S1 | deviation | §6.4 l.1032 / §3 l.571 "Positions come before labels"; spread positions "come next in the numbering" | Any data *value* or data *type* mixing positions and labels is rejected, including `(0, ..t)`; the same mix in call data is accepted | E0101 at `crates/ilang-syntax/src/facts/pre.rs:710,1017`; my re-run `x : (Int, a = Int) = (1, a = 2)` → 2× E0101; `f(1, port = 2)` works | 让数据值与数据类型接受位号+标签混写和任意位置 spread（与调用实参一致）。 | impl | 未测（全仓数据字面量与类型） | syntax | B12a、B12b |
| B4 | S1 | deviation | §9.2 l.1371 "Type checking makes certain that every service is there when a call takes it" | `i check` accepts an entry whose need nobody provides; only build/run reports E0523 | G/s3: `main` needs `Clock`; `check` exit 0, `run` E0523 | `i check` 跑入口 need 检查；或改 §9.2 明说只有 build 检查。 | impl | unmeasured | check | — |
| B5 | S3 | deviation | §11.8 l.2660 "`-.` comes first for a negative float, including negative zero, whose text is `-.0.0` … the text can be read as a Float expression" | `-` is used: `inspect(-.1.5)` → `-1.5`, negative zero → `-0.0`; `float_text.to_string` too | I/f1,f2; my re-run confirmed; `stdlib/src/float_text.i:34 | 运行时与 `float_text` 输出 `-.`；同步改运行时测试的期望值。 | impl | 运行时 1 处 + float_text 1 处 + 6 条测试期望 | runtime | — |
| B6 | S3 | deviation | §6.7 l.1064 "`v.f` and `v.k` read a key of data, or of the data of a labelled value" | `.k` on a labelled value is rejected | E0512 for `Pair(1,2).0` and `Ok(5).0`; `.f` works (E/a4) | 允许标签值上的 `.k`；或从 §6.7 删除“标签值的 `.k`”。 | impl | unmeasured | check | — |
| B7 | S3 | deviation | §2.4 l.351 "every member [of `A(..)`'s expansion] must be a labelled type defined in this module, or it is a compile error" | Accepted with no diagnostic for `X = Int`, `X = A \| Int`, `X = Option(Int)` exported as `X(..)` | B/e_dotdot_*: exit 0, no output | `(..)` 用于非本模块标签类型时报错；或写清 `(..)` 被忽略。 | impl | unmeasured | check | — |
| B8 | S3 | deviation | §9.2 l.1365 "A provided item is keyed by its known type, which must be one named service type" and l.1317 "A service is … a value of a labelled type" | `1 <- (5)` is accepted silently (a union is correctly E0225) | G/s11: check and run exit 0 | provide 非服务类型报错，或列举允许的内建类型。 | impl | unmeasured | check | — |
| B9 | S3 | deviation | §9.2 l.1365 and §9.3 l.1453 promise warnings ("a provided key that the need does not have"; a `Fail` captured by a function or put in data) | Neither warning exists; no warning code in `crates/ilang-check/src/lib.rs:127-132` | G/s5, G/s11, G/e3, G/k5 (check exit 0, empty) | 实现两条警告并给码；或删 §9.2/§9.3 的承诺。 | impl | unmeasured | check | — |
| B10 | S3 | deviation | §3 l.575 "`<- N` with a bare `UpName` … `N` must be a type parameter in scope" | `<- Int` is accepted | C/s23; my re-run `f : (Int) <- Int -> Int = (n : Int) { n }` → exit 0 | `<-` 后裸 UpName 必须是作用域内类型参数，否则报错。 | impl | unmeasured | check | — |
| B11 | S3 | deviation | §4.3 l.675 "`Opaque` anywhere else, and a type named `Opaque`, are compile errors"; §3 l.577 "`Opaque` is not a type" | `Opaque = Int` / `= ()` / `= Opaque` all define fine; afterwards `Opaque` as a type is E0202 | C/s10b–d | `Opaque` 仍是保留字：`Opaque = …` 报错。 | impl | unmeasured | check | — |
| B12 | S3 | deviation | §6.12 l.1158 "A chain with several groups has the type `{ B1 \| … \| Bm } <- N -> R` … Calling such a value with data of shape `S` uses the group of shape `S`" | Calling an anonymous multi-shape chain directly is E0501; the same chain bound to a name works | E/3: `((p : Int) { p } & (p : Int, q : Int) { q })(7)` → E0501 "not a subtype" | 多形 chain 的直接调用按 `{B1\|…\|Bm}` 的参数检查。 | impl | unmeasured | check | — |
| B12a | S1 | deviation | §7.1 l.1196 "A data pattern … ends with `..`, which leaves out the other keys, or `..r`, which binds them as data" | A rest in a plain data pattern is rejected (`E0101: a rest .. in a data pattern is not supported here`); a **named** rest after a constructor (`P(a = x, ..r)`) is rejected too; only bare `..` after a constructor and list rests work | F/rest*, `crates/ilang-syntax/src/facts/pre.rs:888,863` | 实现数据模式 `..`/`..r`；或删 §7.1/§7.2 的相应能力（优先实现，P13 已排期）。 | impl | 0 实现（本就拒绝）；删规范句 | syntax | — |
| B12b | S3 | deviation | §7.1 l.1198 "`User(name, age)` is a shape error. There is no shorthand for `name = name`" | Accepted, binding by label (`U(age, name)` gives `age = 3`) | F/shorthand; `crates/ilang-check/src/types/pattern.rs:35-45` | 实现标签简写，或在 §7.1 承认它。 | impl | 未测 | check | B3 |
| B12c | S4 | deviation | §7.4 l.1240: a non-covering literal group is reported "as this error alone", with a sample value | For a union subject a missing member gives `C is not a subtype of A \| B` *and* the coverage error; a single guarded branch gives `Int is not a subtype of Never` plus coverage | F/cover* | 未覆盖的 union/guard 情形也只报 coverage 一条。 | impl | unmeasured | check | — |
| B12d | S4 | deviation | §8.1 l.1271 "the format gives the type, so the pattern of a segment may not be a type test `x : T`" | Rejected as required, but with `expected a name, found Int`, which never mentions formats or type tests | F/`<<x : Int>>` | 给位串段的 `x : T` 一条专诊断。 | impl | unmeasured | syntax | — |
| B13 | S1 | deviation | §2.1 l.252 "If modules import one another in a circle, it is a compile error" | The E0214 is right, but with a std dependency the output also contains ~20 unrelated errors inside `stdlib/src/test.is` and a wrong "old syntax" hint | B/i_cycle: `grep -c test.is` → 20 | 循环导入失败时只报本包错误，抑制库模块连锁错误。 | impl | unmeasured | package | — |
| B14 | S3 | deviation | §9.5 l.1482–1507 `Scope` lifecycle, `resource.scoped`, 2-argument `acquire` | Implemented at `2c2923f8a` (`stdlib/src/resource.i`), **not** in the probe binary (old 3-arg `acquire`, no `Scope`) | G/f4–f6 are against the old binary; source shows `Scope = (attach, gate)`, `scoped`, 2-arg `acquire` | 按目标修订复核 `Scope`/`scoped`/两参 `acquire` 的设备测试。 | verify | 0（仅复核） | runtime | EF11 |
| B15 | S4 | deviation | §3 l.578 "breaks directly in type groups are compile errors" | A break inside `{ }` in a type is accepted | C/s11: `x : {⏎ Int ⏎} = 1` → exit 0 | 类型组内换行报错；或改 §3 允许。 | impl | unmeasured | syntax | — |
| B16 | S4 | deviation | §3 l.572 "every member of `T` … must be a data type" for `{ T } -> R` | `{ Int } -> Int` type-checks; the mismatch is reported later as a subtype error | C/v6: E0501 `Int` not `<:` `(Int)` | `{T}->R` 的成员不是数据类型时先报形式错误。 | impl | unmeasured | check | — |
| B17 | S4 | deviation | §6.11 l.1134 "The names of one parameter group may not be bound twice, and its labels are distinct" | `(x, x)` gives the right E0218 **and** a spurious E0219 about labels | E/6 | 位置项重名时不再发标签 E0219。 | impl | unmeasured | check | — |
| B18 | S4 | deviation | §6.12 l.1160 "A branch that can never be used gives a compile warning" | With subject type `(3, 4)`, W0501 lands on the *first* branch and says "matched by an earlier clause", though none is earlier; the reason is the subject type | E/7 | 按“主体类型使分支不可达”给 W0501 并改文案。 | impl | unmeasured | check | — |
| B18a | S4 | deviation | §4.8 l.893 type tests are a compile error for a type variable member | The spec's own `Box(Int)` / `Box(String)` example is rejected as required, but a **false** W0501 is also issued on the `Box(String)` clause ("never used"), although it is the only clause that can match those values | D/typetest: E0501 ×2 + W0501 | 类型测试自身出错时抑制 W0501。 | impl | unmeasured | check | — |
| B18b | S4 | deviation | §4.7 l.852 "An `UpName` … that is neither a type nor a type parameter … The message says how to declare it" | The message says to write `f(A) = …`, and the parser then rejects that form (E0101 says to write `<A>(x : A)`) | D/unbound → E0202 with `f(A) = …`; D/fA → E0101; source `crates/ilang-check/src/resolve/native.rs:1556` | 把提示统一成 `<A>(x : A) -> A`。 | impl | unmeasured | check | — |
| B19 | S4 | deviation | §2.8 l.534 lock-file maintenance; §10.2? table: `i check` only parses/resolves/infers | `i check` rewrites `i.lock` (removed a stale line, left a 0-byte file) | B/l_stale | 在 §2.6.2 命令表注明 `i check` 会维护 `i.lock`。 | spec | unmeasured | docs | — |
| B20 | S4 | deviation | §2.2 l.295 lists built-in attributes including `@js`; l.289 requires `@external` **or `@js`** | E0301 lists only `@external`, `@host`, `@deprecated` (no `@js`/`@macro`); E0308 mentions only `@external` | B/a_unknown, B/f_decl_plain | E0301/E0308 的属性清单补 `@js`、`@macro`。 | impl | unmeasured | check | — |
| B21 | S4 | deviation | §2.2 l.295 example `@external(module : "env", "f")` presented as a labelled argument | Colon labels do not exist; the parser says "a label in data is written `label = value`" before the attribute rule is reached | B/a_colon | 例子改成 `@external("module" = …)` 或纯位置写法。 | spec | unmeasured | docs | — |
| B22 | S4 | deviation | §2.3 l.333 `(A, b) = path` "also puts the exported names … into this module" | It also defines/uses the module name `option` from the last path part; a second such import is E0212 on that name | B/i_dupname | §2.3 写明 `(A)=path` 也占用末段模块名。 | spec | unmeasured | docs | — |
| B23 | S3 | deviation | §2.5/§2.3 do not say whether built-in names may be shadowed | `Int = (a = String)` is accepted and silently shadows `Int`; `x : Int = 1` then fails confusingly | B/ty_int | 写明内建名可被顶层名遮蔽；可选加一条警告。 | spec | unmeasured | docs | — |
| B24 | S4 | deviation | §1.2 doc-comment placement | `///` on a statement, `///` at end of file and `////` after items are all accepted with no diagnostic | A/doc_* | 写明错位文档注释是错误、警告还是忽略。 | spec | unmeasured | syntax | — |
| B25 | S4 | deviation | §1.6 "The value of a float literal must be a finite binary64 number" | Overflow is rejected, underflow (`1.0e-400`) is silently 0.0; rounding of subnormals is unstated | A/n_1.0e-400 | 写明下溢与次正规的舍入。 | spec | unmeasured | syntax | — |
| B26 | S4 | deviation | §1.6 "A backslash immediately before a physical line break joins that line" (bullet placed under multi-line strings) | Only works in `"""`; in a cooked `"…"` it is "unknown escape" (the grammar agrees), but the wording reads generally | A/cooked_join | 把“反斜杠换行”限定到多行串。 | spec | unmeasured | docs | — |
| B27 | S4 | deviation | §1.6 "Interpolation is forbidden in … literal types" | Rejected with a generic parse error that never mentions interpolation (patterns get a specific one) | A/lit_interp | 字面量类型里的插值给专诊断。 | impl | unmeasured | syntax | — |
| B28 | S3 | deviation | §2.1 l.20 "A program may not define a module with any of these library paths" | Cannot actually happen: `src/i/io.i` becomes `pkg/i/io`, and neither a package nor a dependency may be named `i` | B/m_libpath | 可删，或保留为一句话说明。 | spec | unmeasured | docs | — |
| B29 | S2 | deviation | §4.7 l.850 `id = <A>(x : A) -> A { 1 }` "is correct" while it makes the §4.8 known-type invariant false | Not reproduced on this binary (D2) | D | 随 A8/T8 一并定。 | impl | unmeasured | check | A8 |
| B30 | S1 | deviation | §14.5/§14.9: an expanded result is ordinary checked code | A macro whose result contains an `@macro` item **panics the compiler** (exit 101) when the template sits past the end of the calling file; otherwise the error shows the template's line/column under the caller's filename | J/crash; `crates/ilang-macro/src/project.rs:454` "expanded diagnostic has an origin" | 修宏展开诊断的 span 归属，消除 panic。 | impl | 0（新增回归） | macro | 无 |
| B31 | S1 | deviation | §14.6 L3034–3035 "A made name of another module reaches only exported names" | Macro-made code reaches a top-level name the macro's module does not export; the probe runs and returns the private value | J/hygiene | 宏生成名的查找只落在导出名上。 | impl | 0（新增回归） | macro | 无 |
| B32 | S3 | deviation | §14.9 L3086 "code from a template is reported at the macro call, with the template as a note" | A result of the wrong form is reported only at the template in the macro module: no call location, no note | J/errloc | 模板代码的错误报在调用处、模板作注（§14.9）。 | impl | unmeasured | macro | — |
| B33 | S3 | deviation | §14.9 L3084: a binding with a quote is a compile-time binding (a quote is a legal value) | Any top-level binding whose value is a quote fails E0322, also in a module no entry imports and when only a macro uses it; the message says "2 errors" and prints one | J/quote | 允许 quote 作顶层绑定值，或给出可用的替代写法。 | impl | unmeasured | macro | — |
| B34 | S4 | deviation | §1.7 L200 "comments keep their place" | `i fmt --to is` swaps a doc comment and the plain comment before it; the code still round-trips and runs | J/fmt-order | `i fmt --to is` 保持文档注释与其前注释的顺序。 | impl | 0（新增回归） | fmt | — |
| B35 | S4 | deviation | §1.7 L200 vs L219 (conversion may refuse rather than drop a comment) | Conversion refuses any file with an end-of-line comment inside a block, both directions, leaving it unchanged | J/trailing | 在 §1.7 写明“转换可因注释位置拒绝，且优先保持注释顺序”。 | spec | unmeasured | fmt | — |
| B36 | S4 | deviation | §11.7 stop text / §4.8 usefulness | A host-service error also carries a spurious "never used" W0501; macro stop traces show `probe/_compiler_macro_0` instead of a source name | J/noise | 抑制 host-service 错误旁的 W0501；停止栈用源名。 | impl | unmeasured | check | — |

## 3. Spelling and consistency decisions (KA–KH)

These rows compare two spellings of one thing. Most are not about the type
system but about whether the language has one rule or several. `justified
exception` rows go into the §0 list of exceptions and change no code.

### KA. Names, paths, imports

| ID | Sev | Phenomenon | Other spelling / rule | Basis | Class | Decision | Work site | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| KA1 | S1 | **原始路径 `a/b.C` 在所有 10 个非 import 位置都被拒绝**，且**每个位置给不同错**：①表达式→静默变除法（有 `qual=4`、`mm=(c=2)` 时 `x = qual/mm.c` 通过且 `x : Int`；否则 E0201 两次）；②类型标注 E0101「expected an item…found `/`」；③模式 E0101「expected `,`, `?` or `)`」；④类型实参 E0101「expected `,` or `)`」；⑤服务项 E0101「expected a named type, found `a`」；⑥need 项同⑤；⑦属性名 E0101；⑧类型定义头 E0101/E0102；⑨导出项 E0101「expected `,` or `)`, found `.`」；⑩导入成员表 E0101；Lisp 侧表达式另报 E0101「`/` is not an expression form with these parts」 | 同一件事"用模块成员"；导入后 `a.C` 到处合法，只有原始 `/` 被拒，且拒绝形态各异 | spec §3 `NamedType = [name "."] UpName`、§4.2 `core.Int`；`parse.rs:989`（值）、`1467/1684`（类型）、`1249`（模式）、`459`（导入名）、`495`（导出）、`615`（类型定义头）、`1566`（need）、`rows.rs:883/939`（Lisp）；`legacy.rs:1694`（E0207） | 真+诊 | ①保留"路径只在 import"，在 §3 写一条明确句子；②所有 10 处共用一句诊断：「a module path is written only in an import; import the name and use it bare」，附修法 `(C) = a/b`；③值的除法退化最隐蔽，需在表达式位置专门拦 `/`；④§4.3 删「`m.T(…) = v` 也是 value binding」的误导 | syntax 一个共用 helper + 10 处调用；spec §3/§4.3 | 否 | syntax+docs | — |
| KA2 | S1 | 同一"把模块当命名空间"有三种拼法：`(clock = i/clock)`、`(i/clock)`、`((Clock) = i/clock)`；第三种还会**隐式**再引入模块名 `clock`，所以 `(i/clock, (Clock) = i/clock)` 报 E0212 `clock` 重复 | 三者都让 `clock.sleep_ns()` 可用，用户无依据选；而 `(clock) = i/clock`（成员名当模块）反而不行，因为 `clock` 未被导出 | spec §2.3 `ImportItem`；`parse.rs:448–483`、`rows.rs:904–930`；隐式命名与显式同名共用一张表；E0212 `lib.rs:37` | 真 | 只留**显式别名**：`(clock = i/clock)`。删自动名（必须写别名）；成员表只带成员、不再隐式命名；成员可改名 `(Clock2 = Clock) = path`；可合写 `(clock = (Clock) = path)` | 语法+check；477 文件 1107 行自名导入、174 文件补别名；spec §2.3 重写 | 是 | syntax+docs | — |
| KA3 | S2 | 同一模块导入两次：两个不同别名 `(k = i/clock, j = i/clock)` 通过；同名两次 E0212 | "同一模块能否导入两次"无规则 | `parse.rs:448`、`legacy.rs:1184`（只按名字查重） | 真 | 按"名字是唯一键"：同一模块可多名，同名必错；写进规范 | spec 一句 | 否 | syntax+docs | — |
| KA4 | S2 | 成员不能改名：`(Clock2 = Clock) = a/b` 报「two patterns joined by `=` must both be names」；两个模块导出同名类型时只能给模块起别名，而路径禁用后又得全仓改 | §2.3 只允许整模块别名；没有 `(name = Member)` | `parse.rs:459–465`、`rows.rs:908–913` | 真 | needs owner（选项 1：允许 `(Name = Member) = path`；选项 2：只允许模块别名） | syntax+resolve | 否 | syntax+docs | — |
| KA5 | S2 | 局部值遮蔽模块名：值位置静默改读字段（`a = (x = 2)  a.x` 通过），类型位置报 E0207「不是模块；局部名覆盖模块名」 | §2.3 说类型与值在**不同**命名空间，值 `a` 本不该挡住 `a.Clock` | spec:336；`legacy.rs:1692–1702` | 真 | 类型位置先按模块解析（`m.T` 的 `m` 只可能是模块），删 E0207 遮蔽规则；值位置维持字段优先 | check 一处；spec 一句 | 否 | syntax+docs | — |
| KA6 | S4 | 导出表不能写导入来的名字：`(Clock) = a/b` 后 `(Clock)` 报 E0215「not defined」；模块名也不能导出 | 重新导出是常规需求；§2.4 又要求"必须在本模块定义" | `parse.rs:495`、`rows.rs:939–956`、`legacy.rs:1006` | 理（未写清） | 保留"只导出本模块定义"，在 §2.4 明说；重新导出须写本地别名定义 | spec 一句 | 否 | syntax+docs | — |

### KB. Brackets and grouping

| ID | Sev | Phenomenon | Other spelling / rule | Basis | Class | Decision | Work site | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| KB1 | S2 | `g = { show }` 丢分支表，`g(1)` 推成 `String\|Int`；`g = show` 不丢 | §6.12 明说"after removing groups"；§6.4/§6.10 说 `{e}` 透明 | spec:1132；`types/groups.rs:86,260` 只认 `ExprKind::Fn/Name` | 冲 | 实现 ungroup 后再判表；规范不动 | check 一处 | 否 | syntax | — |
| KB2 | S2 | `1 \|> { f }` 报 E0111，且提示"把别的放进 group" | `{ f }(1)` 通过，证明组在被调用处透明 | spec:1069；`parse.rs:138,792` | 真 | `pipe_target` 先剥组；错误文案只建议给左操作数加组 | syntax 一处 | 否 | syntax | — |
| KB3 | S2 | 规范 §4.3 说"以 `{` 开头的 item 是 value binding"，`{ x } = 1` 在顶层和块内都报 E0101 | 模式文法里没有 `{` | spec:683 vs §7.1:1149；`parse.rs:513` | 冲（规范自相矛盾） | 从 §4.3 删 "or `{`" | spec 一句 | 否 | syntax | — |
| KB4 | S4 | `-{1}` 在表达式/类型里是 `-1` 字面量，在模式里 E0108 | §6.3、§3 的 IntPart 都允许组 | spec:1009,554；`parse.rs:1292` | 理（模式无组） | 在 §7.1 写一句"patterns have no groups"；不改实现 | spec 一句 | 否 | syntax | — |
| KB5 | S4 | `f = (x : Int) -> Int \| String {}` 通过；`f : (Int) -> Int \| String = …` 报 E0101 | 同一段 `-> Int \| String`，函数体的 `->` 读 `Union`，`:` 后读 `Annot` | spec:568,981；`parse.rs:1148` vs `1436` | 理（`\|` 在参数里还是备选） | 列进 §0 例外清单，不改语法 | spec 一句 | 否 | syntax | — |
| KB6 | S4 | `<- (T)` 里 `(T)` 是服务集合，不是数据类型；`<- Fail(String)` 报「expected `->`」 | 别处 `(T)` 必是数据 | spec:545,154；`parse.rs:1551` | 理（need 无序） | §3 明写"`<-` 后 `()` 是集合"；给 `<- UpName(` 一条专诊断 | spec+syntax | 否 | syntax | — |
| KB7 | S4 | 文件首 `(main)` 是导出组，条目之后写报 E0115，块内是一位置数据 + W0502 | 同一 `(name)` 三义 | spec:271–277；`parse.rs:521–527` | 理 | 列进 §0；不需改 | spec | 否 | syntax | — |
| KB8 | S3 | 空 `{}`/`() {}` 报 E0105「用 core.never」，空类型组也报错；`x : { } = 1` 的错落在 `:` | 空形式应一致 | `parse.rs` 空块 | 理（诊断差） | 诊断指向空组本身 | syntax | 否 | syntax | — |

### KC. Constructors and calls

| ID | Sev | Phenomenon | Other spelling / rule | Basis | Class | Decision | Work site | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| KC1 | S4 | 模式 `U()`（零数据构造器）**静默通过**，表达式 `U()` 报 E0501「`U` is not a subtype of `() <- _ -> _`」 | §6.7、§7.1 都写"`C()` is a compile error" | spec:1030,1163；`parse.rs:1386–1395`（空 args 照收）、`resolve/three.rs:3149`（`args.iter().flatten()` 把 None/Some([]) 合并） | 冲 | **删掉**"`C()` 错误"，统一为合法简写：`C()` 与 `C` 同义（值/模式都一样）；种类仍由 `C` 是值决定 | check 调用/模式解析；全仓无需改。目标修订已改为 `C()` 合法、`C` 报 E0522，本行留作记录 | 否 | syntax+check | — |
| KC2 | S1 | `f(..t)` 只许构造器调用，`1 \|> f(..t)` 报 E0206「`f` is not a constructor」；`W(..d)`（`d` 是普通数据）报「`(m = 2, n = 1)` is not a subtype of `W`」；`f(n = 1, ..(m = 2))` 报「`..` is the first item of a constructor call」 | §6.9 的原例就是 `a \|> f(..t)`；§6.4 说 spread 可用数据 | spec:1015,1030,1069；`three.rs:2149–2172`、`infer.rs:2218`、`facts/pre.rs:692–697` | 冲+真 | 删 record-update 特例：调用括号就是 Data，spread 一律按 §6.4；首项不再限制 | syntax+check，中 | 否 | syntax+check | — |
| KC3 | S4 | `C(..)` 零数据构造器可用（spec 未写） | — | `parse.rs:1386` | 理 | 随第一行一并写清 `C()`/`C(..)` | — | 否 | syntax+check | — |
| KC4 | S4 | 零数据构造器只是值、不是函数：`k(U)`、`() \|> U`、`t : () -> U = U` 都 E0501；普通函数 `f` 可作函数值 | 元数 ≥1 的 `W` 可作函数值 | spec:23,672,1009；`infer.rs` 构造器类型 | 理 | 若采纳 `C()` 简写，语义仍是值；把这条列进 §0 | spec 一句 | 否 | syntax+check | — |
| KC5 | S3 | `h = W & (s : String) { 0 }` 报「`&` joins only function literals: a function literal … must follow it」，后面明明跟着函数字面量 | 规则本身正确（链只由字面量构成），文案错 | spec:992；`parse.rs:830–834` | 诊 | 文案改「the left side of `&` must be a function literal」 | syntax | 否 | syntax+check | — |
| KC6 | S3 | 私有构造器：`other.U` 报 E0204；导入 `(U) = other` 后**值** `U` 报 E0201「not defined」，同名类型 `U` 却可用 | §2.3「导入 UpName 会带来类型和构造器，if they are there」 | spec:333,347；resolve 导入路径 | 诊 | 值位置报「constructor `U` is not exported (write `U(..)` in exports)」 | check | 否 | syntax+check | — |
| KC7 | S4 | `mk = Pair  (b, c) = g` 是一条绑定、会拆 `g`，额外给未文档化的 W0506 | §1.7 有行规则，但没有 W0506 | spec:23,159；`parse.rs:1724` | 理 | §1.7 写明 W0506；§0 说明行规则属于词法层 | spec | 否 | syntax+check | — |

### KD. Patterns and matching

| ID | Sev | Phenomenon | Other spelling / rule | Basis | Class | Decision | Work site | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| KD1 | S1 | 绑定写了类型 `show : {…} = A & B` 就丢分支表（`show(1)` 按 `String\|Int`），不写类型有表 | 同一个链、同一个值 | spec §6.12；`three.rs:3315` 只认 `PatternKind::Var` | 真（规范自认但没进 §0） | **删**"只有裸名字才有表"：写类型照样保表，类型只作上界；E0501 附"因写了类型"提示 | check 推断 5a/three | 否 | check | — |
| KD2 | S1 | `g = { show }` 丢表（见 B 第一行） | spec 写"after removing groups" | 同上 | 冲 | 实现 ungroup | — | 否 | check | — |
| KD3 | S4 | `x : T` 是标注还是测试取决于"单分支且无 guard"；`v \|> (x : Int) {x}` 对 `{Int\|String}` 报 E0501 | — | spec:23,1170；`infer.rs` fitted vs rule 11 | 理（规范自认） | 保留，但在 §0 列全三个条件并说明代价 | spec | 否 | check | — |
| KD4 | S2 | 规范又说"有 alternatives 就是测试"，可 `(x : Int \| x : Int)` 收 `{Int\|String}` 仍走 fitted、报 E0501 + Assert | 规则 11 只管多分支/有 guard | spec:1170 vs §6.12:1124、§4.8 规则 4/11 | 冲 | **删** alternatives 条件：只看组是否被规则 11 定型。§0 例外从 3 条降到 2 条 | spec 两句；实现已是 | 否 | check | — |
| KD5 | S2 | 字面量模式 `"a" = s` 合法（放宽成 `String`、可能失败、注入 Assert）；等价标注 `s \|> (x : "a")` 报 `String` is not a subtype of `"a"` | 两者匹配集合相同 | spec:1087,1193；`infer.rs` fit | 真 | 让字面量也按字面量类型 fit（删"fitted 时放宽到整个 Int/String"）；要失败语义就写 match | check 的 fit/coverage；spec §7.3/§7.4；迁移 `"a" = s` 类 | 是（少量） | check | — |
| KD6 | S2 | 参数里写了单值类型仍要 Assert：`f : ("a") -> Int = ("a") { 0 }` 报「Assert is not a subtype of Never」；绑定 `"a" = x`（`x : "a"`）不要 | 同上派生 | spec:1193；`infer.rs` | 真 | 随上一行消失 | 同上 | 同上 | check | — |
| KD7 | S3 | 同一"需要 Assert 但不允许"：绑定/匹配报在模式处并说明原因；参数组报 1:1 只有子类型错 | — | `infer.rs:3926–3935,607` vs `add_env` | 诊 | 两条路径都走 `defer_failure`/`binding_failure` | check | 否 | check | — |
| KD8 | S3 | 绑定里写 `1 \| 2` 报「`\|` no longer joins function values…」；嵌套数据/列表项报「expected `,` or `)`」 | alternatives 只许在参数项 | spec:1170；`rows.rs:1471` | 诊 | 统一「alternatives are only allowed in parameter items」 | syntax | 否 | check | — |
| KD9 | S2 | 数据模式的 `..` 全被拒（`(a, ..) = v`）；构造器的裸 `..` 可以；`..r` 处处被拒 | §7.1/§7.2 表格定义了 `..` 与 `..r` | spec:1164,1195；`facts/pre.rs:888,863` | 冲 | 按"优先删规则"：从 §7.1/§7.2/§6.11 删掉数据模式 `..`/`..r` 与"唯一已知形状"，只留构造器 `..` | spec 数句（实现本就拒） | 否 | check | — |
| KD10 | S2 | 标签项里不能写子模式：`Box(value = 1, tag = t)` 报 not supported；位置项可写 `((1, t))` | §6.11「`label = p` matches the part」 | `facts/pre.rs:926–935` | 冲+真 | 放开，标签项与位置项同一套 | syntax+lowering+coverage | 否 | check | — |
| KD11 | S2 | 合取最多一个结构部分：`(a,b) = (c,d) = v` 报「two patterns joined by `=` must both be names」 | §6.10/§7.1 说合取是任意多个 part | spec:1075,1147；`pre.rs:908–925` | 冲 | 规范写明"至多一个非名字 part"（诚实删能力） | spec 一句 | 否 | check | — |
| KD12 | S4 | guard `?` 只在参数组；绑定里写报「expected `,` or `)`」；`a <> "x" <> b` 报「expected `,`, `?` or `)`」 | — | spec:1101,1197；parse | 理（无回退分支/多义） | 只改诊断 | syntax | 否 | check | — |

### KE. Type definitions and type syntax

| ID | Sev | Phenomenon | Other spelling / rule | Basis | Class | Decision | Work site | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| KE1 | S1 | 完全相同的头 `Box(X) = …`，右侧是值类型时 `Box` 是**用**已有类型、`X` 定义成模块级新名字；否则 `Box` 是**定义**、`X` 只是局部参数。影响往回传：`Pair(A, B) = (A, B)` 加一行 `Pair(A, _) = p` 后，第一行报 E0231「参数不能叫类型」 | §0 只见 §7.1 一个例外 | spec:663–681,828；`parse.rs:620`、`ast.rs:253`、`three.rs:561,1206` | 真 | needs owner（选项 1：删除别名模式，只留 `T = v` 依赖值的名字；选项 2：保留并修 §4.3 第 1 条） | syntax+check+spec；tests/spec/4.3 约 132 文件 | 是（测试为主） | check+syntax | — |
| KE2 | S1 | `T = v`（无头）被实现当作"定义依赖值的名字"，按 §4.3 第 1 条它又该是别名模式、`T` 须已存在 | §4.3 只写 `T(P)=v`；§14 又提 value-dependent alias | spec:665,2989；`ast.rs:253` | 冲 | 修 spec 第 1 条为"头部非空且右侧是值类型"；保留无头 `T = v` 作唯一种 | spec | 否 | check+syntax | — |
| KE3 | S2 | `_` 四义：模式 discard、类型推断洞、别名模式"跳过"、差集右侧"只看构造器"；前三种可同文件共存并通过 | §0 同写法同规则 | spec:557,680,723；`parse.rs:1710` | 真（第一种） | 删别名模式后"跳过"消失；差集右侧算洞的特例，保留 | 随第一行 | 随第一行 | check+syntax | — |
| KE4 | S4 | `T = (Int, Int)` 是新 labelled 类型；`T = { (Int, Int) }` 与 `T = (Int, Int) \| (Int, Int)` 是别名；值集合相同但互不可赋值、有无构造器不同 | 只差外层括号/分组 | spec:663–676；`parse.rs:629–641` | 理 | 保留（"新类型"必须有写法）；可选：要求归并后只剩一个数据类型的 union 报错并让写 `{D}` | 小 | 否 | check+syntax | — |
| KE5 | S3 | `x : Int \| String` 在 4 个位置 4 种错：顶层 E0101「expected an item…`\|`」、参数 E0201 `String` is not defined + E0220、块内 E0101「expected an expression, found `:`」、`(v : …)= 1` 报「label is written `label = value`」 | 同一 Annot 规则 | spec:568；`parse.rs:1436,1552,899` | 诊 | 4 处统一「`:` 后的类型在 `\|` 处结束，写 `{ A \| B }`」 | syntax | 否 | check+syntax | — |
| KE6 | S2 | 诊断推荐已删语法：`f(A) = …`（§2.5 明说编译错误）、`Box(A) { … }`（`parse.rs:623` 已拒） | 同一"怎么声明类型参数" | `three.rs:1556,1558,1202`、`legacy.rs:2129,1398` | 冲 | 改 `f = <A>(x : A) -> A { x }`、`Box(A) = (A)` | check 5 个字符串 | 否 | check+syntax | — |
| KE7 | S2 | `Phantom(A) = ()`、`T(A) = Opaque` 被 E0221「参数未使用」拒绝 | spec 明确允许（§4.5 就拿 `Phantom` 举例） | spec:674,749,769；`three.rs:1268–1279`、`legacy.rs:1458` | 冲 | 只对"名字"检查参数使用 | check 两处 | 否 | check+syntax | — |
| KE8 | S2 | 数据项/参数标注里的多态函数类型 `<A>(A) -> Nil` 报「must be the type of a binding」；spec 自己的例子 `Show = (write = <A>(A) -> Nil)` 过不了 | §4.7 允许 | spec:832,794；`pre.rs:966–972` | 冲 | 收窄 spec 到"只在绑定标注"，删 §4.6 后半；实现 rank-N 代价大 | spec | 否 | check+syntax | — |
| KE9 | S2 | `-` 右侧写具体类型参数被接受且产生约束：`{List(Int)\|Int} - List(String)` 报 `Int` is not subtype of `String` | spec 要求右侧参数必须是 `_` 且不加约束 | spec:723；`three.rs:1643` | 冲 | 按 spec 拒；`-` 只剩"看构造器"一种语义 | check 一处 | 可能（grep `- X(`） | check+syntax | — |
| KE10 | S3 | `T = { Opaque }`、`T = Opaque \| Nil` 报「type `Opaque` is not defined」+ 过时提示 | §4.3 要求专门错误 | spec:671；`parse.rs:629–635` | 诊 | 专诊断「`Opaque` 只能作类型定义整个右侧」 | syntax | 否 | check+syntax | — |
| KE11 | S4 | 大写名字不能是值：块内 `Clock = 1` 是构造器**匹配**（要求已有 Clock），顶层 `Clock = 1` 是类型定义 | §1.3 说 UpName 是类型/参数/构造器（已文档），但 `x = 1` 与 `X = 1` 同形异义 | spec:73,281；`parse.rs:531` | 理 | 保留；在 §2.2 写一句"`UpName = …` 是类型定义，值名用构造器/小写" | spec | 否 | check+syntax | — |

### KF. Needs and services

| ID | Sev | Phenomenon | Other spelling / rule | Basis | Class | Decision | Work site | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| KF1 | S1 | row 变量有三种写法都过：`<- N`、`<- (..N)`、`<- (N)`；且 `<- Clock`（具体服务）也被接受并当具体服务（后续 `Clock is not a subtype of Never`） | §3：裸 `<- N` 必须**是**类型参数；函数头只认 `(..e : N)`，裸写 E0108 | spec §3,§4.7,§6.8；`three.rs:1707,1703`、`parse.rs:1566` | 冲 | 裸 `<- X` 只许类型参数；`(N)` 里是类型参数就报「写 `(..N)`」；保留 `<- N` 简写（语料 4199 处 `<- N ->`） | check 两处；`docs/design/std-v3.md` 若干处 `<- C` 要改 | 少量 | syntax+check | — |
| KF2 | S1 | 同一 need 列表重复键：类型里静默合并（完全相同的重复也不警告）、函数头 E0228、provide E0225 | §9.2 说三处都"keyed by the type constructor" | spec §3 末、§9.2；`three.rs:2743`、`infer.rs:3636` | 真 | 手写重复一律错；合并只留给推断；删 §3「Two items … are merged」 | syntax/check 一处；spec 一句 | 否 | syntax+check | — |
| KF3 | S1 | 空 `<- ()`：函数头 E0108、类型 E0101、provide `e <- ()` **静默通过** | §3/§6.8「`<- ()` is not written」 | `parse.rs:1132`(E0108)、`1605`(E0101)；provide 无检查 | 真+诊 | 三处同一条诊断；提供一个错误码 | syntax 统一 + check 一处 | 否（一测试串） | syntax+check | — |
| KF4 | S1 | rest 位置四处四规则：服务模式 `..env` 必须最后（E0107）、类型 need 任意、provide 任意、数据表达式必须是第一项 | §9.2「order does not count」 | spec:1139,§6.4；`parse.rs:1139`、`pre.rs:710` | 真 | 服务模式删"last"；数据表达式随 C 组改成任意位置 | syntax 两处 | 否 | syntax+check | — |
| KF5 | S3 | 同一"此处不许带 need"四种报法：`Clock is not a subtype of Never`、`Assert is not…`、绑定版有原因说明、顶层 E0509 | — | `infer.rs:1580,607,558` | 诊 | 统一「this function needs `X`, but the expected type allows no need」+ 来源（服务模式/可能失败/调用） | check | 否 | syntax+check | — |
| KF6 | S3 | `e <- (s) + 1` 报「expected an expression, found `+`」 | §6.9 专门规定 provide 右操作数只能是 postfix | spec:1070；parse 通用路径 | 诊 | 专诊断并建议加组 | syntax | 否 | syntax+check | — |

### KG. Unions, intersections, records, host null

| ID | Sev | Phenomenon | Other spelling / rule | Basis | Class | Decision | Work site | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| KG1 | S1 | 值层 `&` 是"合并分支"：`k = (Up){1} & (Down){"a"}` 得 `(Up\|Down) -> 1\|"a"`；类型层 `&` 是交：同一对函数类型规范化成 `(Up\|Down) -> Never`。用类型标注这个 chain 就报 E0501 `1` is not a subtype of `Never` | 同一符号两个完全不同的运算；chain 的类型也不等于分支类型的交 | spec §6.12 vs §4.4；`solve.rs:2440–2481`（并与交共用 merge，参数走 `merge_arg`） | 真 | needs owner（选项 1：换掉 chain 分隔符，`&` 只剩交；选项 2：只写清“chain 的类型不是分支的交”并加专诊断） | 全仓 chain 与解析器/着色（刚迁移过 `&`，代价高）；最低版只改 spec+一条诊断 | 是（全仓）/否（最低版） | check | — |
| KG2 | S2 | 不变参数在交里不相等：spec 说结果 `Never`、"不是编译错误、不强制相等"；实现对并与交都调 `equal`，`(x : {Cell(Int)} & {Cell(String)})` 报 E0503 | §4.4 第 4 条 | spec:731；`solve.rs:2476–2479` | 冲 | `!union` 且不等时返回 `Never`，不 `equal`；删一个特例 | check 一处 | 否 | check | — |
| KG3 | S3 | 同一 `A\|B` 的三种合并：协变 `Box` 得 `Box(1\|"a")`、逆变 `Sink` 得 `Sink(Never)`（不报错）、不变 `Cell` 报 E0503，且消息显示 `_` and `_` | §4.4 三套规则本身正确 | spec §4.4；`solve.rs:2470–2481`、`infer.rs:1582` | 理（规则）+诊（消息） | 规则不动；消息显示实参或指出两个来源分支 | check 消息 | 否 | check | — |
| KG4 | S2 | 两个 `Opaque` 类型在编译期是不同构造器，`E1 \| E2` 作为 sum 被接受；但类型测试把所有 opaque 当一个构造器，于是永远匹配不了；报错还泄露 `i/big.Digits`、`i/syntax.*` | §4.3 sum 要求不同构造器 vs §7.1 末段 | spec:697,1208；实验 `opq` | 真 | 把"所有 Opaque 算一个构造器"提前到 §4.3 sum 校验，定义处就报；差集消息只列用户写出的类型 | check sum 校验 + spec 一句 | 否 | check | — |
| KG5 | S2 | `User(..u, age = 3)` 有独立节点 `RecordUpdate`，要求 `u : User`；`User(..d)`（同形状普通数据）报 E0501 | §6.7 说这就是"普通调用 + 数据 spread" | spec:1030；`hir.rs:492`、`infer.rs:2218,3223` | 真+冲 | 删 `RecordUpdate`，按 spec 当普通调用处理（见 §1C 第二行） | syntax+hir+infer+codegen | 否 | check | — |
| KG6 | S2 | 数据 spread 用 labelled 值 `x = (..u)`（`u : User`）报 E0506「…is data, but it has `User`」 | §6.4 明文允许"或构造器在此可用的 labelled 类型" | spec:1011；`infer.rs:2075–2093` 只认 `Head::Shape` | 冲 | 按 spec 实现并做 §2.4 可见性检查 | check | 否 | check | — |
| KG7 | S4 | 取负：`-1`/`-{1}` 的类型是 `-1`，`-x` 是 `Int`；`f = (n : -1)` 下 `f(-{1})` 过、`f(-x)` 报 E0501；`[-1,1]` 是 `List(-1\|1)`，`[-x,1]` 是 `List(Int)` | 一元 `-` 的类型取决于操作数是否语法字面量 | spec §6.9/§6.3；`infer.rs:2224–2235`、`known.rs:1056` | 理（负数无别的字面写法） | 不动，保留 §6.9 现有句子 | — | 否 | check | — |
| KG8 | S4 | W0503 只比顶层：`1 == 2` 警告，`(1,2)==(3,4)`、`Some(1)==Some(2)`、`[1]==["a"]` 不警告 | 与 §4.5 规则 3"不相交"同口径 | spec:1074；`infer.rs:2280`、`known.rs:142` | 理 | 不动 | — | 否 | check | — |
| KG9 | S4 | 宿主 JS `null` 作为不透明 externref（`reference_id` 0）可与 i 值 `==`；i 的"无值"是 `None` | 同一"无值"两套表示，宿主那套无类型 | §10.3 未提；§B.4:3127 | 理（spec 层面；未跑运行时） | 类型里不加 null；在 §10.1.1 写明 null/undefined 是普通不透明值，库提供 `js.is_null` | spec+库 | 否 | check | — |

### KH. The standard about itself, colons, declaration forms

| ID | Sev | Phenomenon | Other spelling / rule | Basis | Class | Decision | Work site | Migration | Owner | Depends |
|---|---|---|---|---|---|---|---|---|---|---|
| KH1 | S2 | 规范开头说"两处看起来一样的写法规则相同，只有 §7.1 一个例外"；实测至少还有 6 条位置规则不在清单里：函数体 `->` 读 Union 而 `:` 后读 Annot；`<- (T)` 是集合不是数据；文件首 `(name)` 是导出组；位串段 `:` 是 Format；`{` 在 `->` 后是类型；行首 `(` 是新语句 | 例外清单自称穷尽 | spec:23；§1.7:169、§3:568、§6.1:993、§8.1 | 真（契约层） | 把清单补全，或改成"例外以各节为准"；删不掉规则，但让用户有单一查阅点 | spec 一处 | 否 | docs | — |
| KH2 | S4 | `:` 到处引类型，唯在位串段引 Format（`<<x:u8>>`）；`x : u8` 是值类型/标注，`<<x : u8>>` 是格式 | §3 明写"this rule does not apply" | spec:136,568,1230；`parse.rs:1019` `segment` | 理 | 列进 §0 清单；不必改语法 | spec | 否 | docs | — |
| KH3 | S4 | `name : Type` 没有 `=` 是声明，但只有 `@external`/`@js`/宏签名合法；同样写法的普通声明报「declaration with no @external is kept for macros」 | 绑定可无标注自由推断；声明必须推断且挂属性 | spec:281–286、§10.1 | 理（宿主边界） | 列进 §0；诊断里给出两个合法修法（加 `= e` 或 `@external`） | syntax 诊断 | 否 | docs | — |
| KH4 | S4 | 小写类型参数 `<a>` 报「expected an expression, found `<`」；只有 `UpName` 可当参数 | 数学/ML 习惯用小写类型变量；§0 自己用 `a` 记类型变量 | spec:73,826；`parse.rs:1056` | 理（`a` 已是值类型） | 列进 §0，或给一条专诊断「type parameters are UpNames」 | syntax | 否 | docs | — |
| KH5 | S4 | 旧二进制未定义 `Nil`，而规范与 `lib/prelude.i` 已无 `Nil` | 目标修订已全仓删除 `Nil`，测试改用 `()` | `git show 2c2923f8a:docs/spec.md` 无 `Nil`；二进制早于该提交 | 冲（构建） | 用目标修订重建即可，规范与 prelude 一致 | verify | 0 | docs | — |

## 4. Deliberate non-changes

Four forms stay as they are; they are recorded so a later audit does not
re-open them.

1. `(e)` is one-position data and `{ e }` is a group; the type level is
   symmetrical. Changing this would touch every call in the repository.
2. `()`, `(x)` and `{e}` keep their three meanings; they are the core shape of
   data and of inference.
3. `Fail(E)` and `Result(A,E)` both stay: one is a need, one is a value.
4. A match through `|>` adds no call frame and passes tail position through;
   coverage checking and tail calls rest on it.

## 5. Method and limits

Six areas were audited in parallel plus a dedicated pass on paths and imports.
Every row records the command and its output in the evidence column, so a probe can
be re-created from the row alone; spelling rows cite `file:line`. The repository
was not changed.

Limits: §10, Appendices A/B, the formatter details, and worker/portability
behavior are not covered. The binary used for behavior is from an ancestor of
`2c2923f8a` and the `target/` directory is shared, so `impl` rows are re-run
after a build of the revision before a fix is merged. `Nil`- and
`i/prelude`-related observations were dropped: both changed at the revision.
