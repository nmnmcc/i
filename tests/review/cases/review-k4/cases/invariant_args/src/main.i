S(A, N) = (A, (A) -> (), () <- N -> (), (N) -> ())
use = <A, N>() <- (s : S(A, N)) {
	S(x, f, g, _) = s
	f(x)
	g()
	x
}
run = (f) { f() }
f = <A, N>() <- (s : S(A, N)) { run(() { use() }) }
