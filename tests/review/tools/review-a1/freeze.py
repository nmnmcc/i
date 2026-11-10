import json,pathlib
root=pathlib.Path('/tmp/review-a1')
cases=[]
def add(name,kind,expected,src):
 cases.append(dict(id=len(cases)+1,name=name,kind=kind,expected=expected,source=src+'\n'))
add('headers_tuple','new','ok','(i/core)\n(f)\n(a, b) = (20, 22)\nf = () { a + b }')
add('old_headers','old','ok','import (i/core)\nexport (f)\nf = () { core.inspect(42) }')
add('mixed_headers','mixed','ok','import (i/core)\n(f)\nf = () { 2 |> { x -> core.inspect(x) } }')
add('header_order','new','error','(f)\n(i/core)\nf = () { 1 }')
add('empty_header','new','error','()\nf = () { 1 }')
add('old_empty_headers','old','ok','import ()\nexport ()\nf = () { 1 }')
add('tuple_first','old','ok','(a, b) = (20, 22)\nf = () { a + b }')
add('attr_tuple_newline','new','ok','@deprecated("old")\n(a, b) = (20, 22)\nf = () { a + b }')
add('attr_tuple_same_line','new','error','@main (a, b) = (20, 22)')
add('macro_attr_tuple','new','macro error','@m\n(a, b) = (20, 22)')
add('attributed_header','new','error','@deprecated("old")\n(f)\nf = () { 1 }')
add('group','old','ok','f = () { { 20 + 22 } * 2 }')
add('first_fn_binding','old','ok','f = () { { go = (n) -> Int { n + 1 } go(41) } }')
add('first_chain_fn','old','ok','f = () { { go = alias = (n) -> Int { n + 1 } go(41) } }')
add('first_typed_fn','old','ok','f = () { { go: (Int) -> Int = (n) -> Int { n + 1 } go(41) } }')
add('body_returns_fn','old','ok','make = (n) { (x) -> Int { x + n } }\nf = () { make(40)(2) }')
add('old_unbound_fn_group','old','E0113 in Both; from-old safe extraction','f = () { { (x) -> Int { x + 1 }(41) } }')
add('old_expected_fn_group','old','E0113; explicit migration refusal','f: () -> (Int) -> Int = () { { (x) -> Int { x + 1 } } }')
add('new_alias_tuple_clause','new','ok','f = () { (20, 22) |> { t = (a, b) -> a + b } }')
add('old_alias_returns_fn','old','ok','g = (p: (Int, Int)) { case p { t = (a, b) -> (x) { a + b + x } } }\nf = () { g((20, 21))(1) }')
add('old_alias_typed_fn','old','safe refusal allowed','g = (p: (Int, Int)) { case p { t = (a, b) -> (x) -> Int { a + b + x } } }\nf = () { g((20, 21))(1) }')
add('new_tuple_returns_fn','new','ok','g = (p: (Int, Int)) { p |> { (a, b) -> (x) -> Int { a + b + x } } }\nf = () { g((20, 21))(1) }')
add('guard_parentheses','new','type error: guard is one-tuple','f = (n) { n |> { x ? (True) -> 1 _ -> 2 } }')
add('guard_fn_grouped','new','ok','f = (v: (Int) -> Int) { v |> { g: { (Int) -> Int } ? True -> g(41) _ -> 0 } }')
add('old_guard_fn','old','ok','f = (v: (Int) -> Int) { case v { g: (Int) -> Int if True -> g(41) _ -> 0 } }')
add('guard_fn_ungrouped','new','error: ? belongs to type','f = (v) { v |> { g: (Int) -> Int ? True -> 1 _ -> 0 } }')
add('param_first','new','ok','f = () { { id(A) = (x: A) -> A { x } id(42) } }')
add('param_later','new','ok','f = () { k = 42 id(A): (A) -> A = (x) { x } id(k) }')
add('param_shadow','new','ok','outer(A) = (x: A) { inner(A) = (y: A) { y } (x, inner(42), inner("s")) }')
add('param_type_collision','new','error','f(Int) = (x: Int) { x }')
add('typevar_call','new','error','f(A) = (x: A(Int)) { x }')
add('call_constructor_boundary','new','ok','Token { Stop }\nsend = (x) { x }\nf = () { send(Stop) }')
add('lower_constructor_pattern','new','error: f is not constructor','f = (x) { x }\ng = () { f(x) = 1 x }')
add('single_assert','new','ok, Assert environment','f = (n: Int) { n |> { 42 -> 7 } }')
add('single_typed_annotation','new','type error','f = (v: Int | String) { v |> { x: Int -> x } }')
add('multi_typed_test','new','ok','f = (v: Int | String) { v |> { x: Int -> x _ -> 0 } }')
add('old_single_typed','old','ok','f = (v: Int) { case v { x: Int -> x } }')
add('single_choices_pending','new','E0235 accepted deferral','f = (r: Result(Int, Int)) { r |> { Ok(v) | Error(v) -> v } }')
add('old_choices','old','ok; split preserves result','f = (r: Result(Int, Int)) { case r { Ok(v) | Error(v) -> v } }')
add('guarded_choices','new','ok','f = (n: Int) { n |> { 0 | 1 ? True -> 42 _ -> 7 } }')
add('old_match_times','old','ok','f = (n: Int) { case n { 0 -> 1 _ -> 2 } * 10 }')
add('old_plus_match','old','ok','f = (n: Int) { 10 + case n { 0 -> 1 _ -> 2 } }')
add('old_match_compare_subject','old','ok','f = (n: Int) { case n > 0 { True -> 42 False -> 7 } }')
add('old_match_concat_subject','old','ok','f = () { case "a" <> "b" { "ab" -> 42 _ -> 7 } }')
add('old_match_call','old','ok','f = (n: Int) { case n { 0 -> (x) { x + 1 } _ -> (x) { x + 2 } }(40) }')
add('old_match_tuple_field','old','ok','f = (n: Int) { case n { 0 -> (42, 0) _ -> (7, 0) }.0 }')
add('old_match_named_field','old','ok','User { User(name: String) }\nf = (n: Int) { case n { 0 -> User("yes") _ -> User("no") }.name }')
add('new_ungrouped_product','new','E0111','f = (n: Int) { n |> { 0 -> 1 _ -> 2 } * 10 }')
add('new_grouped_product','new','ok','f = (n: Int) { { n |> { 0 -> 1 _ -> 2 } } * 10 }')
add('new_match_empty_call','new','ok: pipe inserts argument','f = (n: Int) { n |> { 0 -> 42 _ -> 7 }() }')
add('old_negative_clauses','old','ok','f = (n: Int) { case n { 0 -> 1\n-1 -> 2\n_ -> 3 } }')
add('new_tuple_clauses_lines','new','ok','f = (p: (Int, Int)) { p |> { (0, y) -> y\n(x, _) -> x } }')
add('new_tuple_clauses_joined','new','error','f = (p: (Int, Int)) { p |> { (0, y) -> y (x, _) -> x } }')
add('minus_newline','old','ok: -2 is last statement','f = () { 40\n-2 }')
add('minus_continued','old','ok: 38','f = () { 40 -\n2 }')
add('old_guard_lines','old','ok','f = (n: Int) { case n { 0\n| 1\nif True\n-> 42\n_ -> 7 } }')
add('new_guard_lines','new','ok','f = (n: Int) { n |> { 0\n| 1\n? True\n-> 42\n_ -> 7 } }')
add('old_typevars','old','ok','id = (x: a) -> a { x }\nf = () { (id(42), id("s")) }')
add('mixed_capture_inner','mixed','ok; rename a avoids A','outer(A) = (x: A) { inner = (y: a, z: A) { (y, z) } inner(42, x) }')
add('mixed_capture_outer','mixed','ok; rename b avoids B','outer = (x: b) { inner(B) = (y: B, z: b) { z } inner(42, x) }')
add('old_type_alias_join','old','J1; from-old succeeds','Id(a) = a (p, q) = (20, 22)\nf = () { p + q }')
add('new_record_reserved','new','records error','f = () { { name: 42 } }')
add('new_greedy_type','new','error: no backtracking','f = (v) { v |> { x: Int | 0 -> 1 _ -> 2 } }')
add('empty_clauses','new','ok: Never parameter','f = {}')
add('new_expected_subject','new','ok','User { User(name: String) }\nf = () { (u) { u.name } |> { g: (User) -> String -> g(User("yes")) } }')
add('new_subject_scope','new','ok: outer f used in subject','f = (x) { x + 1 }\ng = () { (x) { f(x) * 2 } |> { f -> f(1) } }')
(root/'corpus.json').write_text(json.dumps(cases,indent=2))
(root/'criteria.md').write_text('''# Pre-implementation acceptance criteria
Frozen after reading syntax.md, spec.md, then a1-front-end.md; no implementation inspected.
1. Accept the language text of syntax.md with only named A1 deferrals: E0113, J1, E0235; old keywords/lowercase vars retained.
2. Header classification/order/nonemptiness and tuple/attribute boundaries follow 2.2 and 1.7.
3. Brace decisions follow the ordered tests of 6.1, including function bodies, chain/function exception, greedy pattern types, malformed-pattern and record diagnostics.
4. Guard arrows end expressions only outside brackets; question marks in function types remain type syntax.
5. Binding type parameters precede call/constructor interpretation; declared variables scope, shadow, generalize and avoid type-name collisions per 4.7.
6. Single-clause matches fit and never generalize; scope, expected types, labels, known fields, Assert and stop positions follow 4.8/6.12. Multi-clause/guarded/empty blocks require coverage. Choices defer only the accepted case.
7. Precedence/postfix/line breaks match 1.7/6.9; migration inserts all needed groups and newlines.
8. Old valid programs keep behavior after formatting, or receive explicit documented safe refusal. No silent captures, changed effects/evaluation/generalization, panic, or non-idempotent successful format.
9. Old-mode parsing matches base. Migration write check compares an independent transformed AST; formatting preserves comments/layout promises.
10. infer.rs/lower.rs unchanged; other edits within documented scope; no weakened tests or unexplained goldens.
11. wat-diff must detect unapproved executable changes, including ownerless code and non-site data, exact errors and export sets.
12. Blind expectations trace to language sentences, with temporary exceptions separated from normative language.
''')
with (root/'programs-before-implementation.md').open('w') as out:
 for c in cases:
  out.write(f"## {c['id']:02d} {c['name']} ({c['kind']})\nExpected: {c['expected']}\n```i\n{c['source']}```\n\n")
print(len(cases),'cases frozen')
