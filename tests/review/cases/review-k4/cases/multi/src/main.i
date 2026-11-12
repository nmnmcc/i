S(A, E, N) = ((A) <- (Fail(E), ..N) -> A, (N) -> Nil)
use = <A, E, N>(a: A) <- (s: S(A, E, N)) {
	S(f, _) = s
	f(a)
}
run = <A, N>(f: () <- N -> A) { f() }
int = () { run(() { use(2) }) <- (S((x: Int) { x + 1 }, (_: Never) { Nil })) }
str = () {
	run(() { use("a") }) <- (S((x: String) { x <> "b" }, (_: Never) { Nil }))
}

main = () {
	int()
	str()
	Nil
}
