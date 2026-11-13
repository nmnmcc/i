S(A, N) = (() <- N -> A, (N) -> Nil)
use = <A, N>() <- (s: S(A, N)) {
	S(f, _) = s
	f()
}
run = (f) { f() }
read = () { run(() { use() }) <> "!" }
with = <X, R, N>(x: X, f: () <- N -> R) {
	() { f() <- (S(() { x }, (_: Never) { Nil })) }
}

main = () {
	g = with(1, read)
	g()
	Nil
}
