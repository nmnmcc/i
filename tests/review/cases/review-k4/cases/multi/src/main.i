S(A, E, N) = ((A) <- (Fail(E), ..N) -> A, (N) -> Nil)
use = <A, E, N>(a: A) <- (s: S(A, E, N)) {
	S(f, _) = s
	f(a)
}
run = <A, N>(f: () <- N -> A) { f() }
int = () {
	_syn3_0: S(Int, Never, Never) = S((x: Int) { x + 1 }, (_: Never) { Nil })
	run(() { use(2) }) <- (_syn3_0)
}
str = () {
	_syn3_1: S(String, Never, Never) = S(
		(x: String) { x <> "b" },
		(_: Never) { Nil },
	)
	run(() { use("a") }) <- (_syn3_1)
}

main = () {
	int()
	str()
	Nil
}
