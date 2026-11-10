S(A, E, N) { S((A) -> A ? E ! N, (N) -> Nil) }
use(A, E, N) = (a: A, s ! S(A, E, N)) { S(f, _) = s f(a) }
run(A, E, N) = (f: () -> A ? E ! N) { f() }
int = () { ! S(Int, Never, Never) = S((x: Int) { x + 1 }, (_: Never) { Nil }) run(() { use(2) }) }
str = () { ! S(String, Never, Never) = S((x: String) { x <> "b" }, (_: Never) { Nil }) run(() { use("a") }) }
@main
main = () { int() str() Nil }
