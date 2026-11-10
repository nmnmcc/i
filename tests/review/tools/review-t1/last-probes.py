from probes import *
run('choices_assert','''(assertion = i/assertion)
f = { 0 | 1 -> "yes" }
@main
main = () {
 assert(f(0)=="yes")
 assertion.catch(() { f(2) }) |> { Error(_) -> Nil  Ok(_) -> panic("missed Assert") }
}
''',True)
run('shared_finite','''Pair(A) { Pair(A,A) }
f = (x: Pair("a" | "b")) { x |> {
 Pair("a","a") -> 0
 Pair("a","b") -> 1
 Pair("b","a") -> 2
 Pair("b","b") -> 3
} }
@main
main = () { assert(f(Pair("a","b"))==1) assert(f(Pair("b","a"))==2) Nil }
''',True)
run('shared_inferred','''Pair(A) { Pair(A,A) }
f = (x) { x |> {
 Pair("a","a") -> 0
 Pair("a","b") -> 1
 Pair("b","a") -> 2
 Pair("b","b") -> 3
} }
''')
run('generic_difference','''drop(A) = (x: A | 0) -> A - 0 { x |> { 0 -> panic("zero")  rest -> rest } }
f = (x: Int) { drop(x) }
g = (x: "a" | "b") { drop(x) }
@main
main = () { assert(f(3)==3) assert(g("b")=="b") Nil }
''',True)
