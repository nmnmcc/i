S(A, E, N) = ((A) <- (Fail(E), ..N) -> A)
use = <A, E, N>(x: A) <- (s: S(A, E, N)) {
	S(f) = s
	f(x)
}
run = <A, N>(body: () <- N -> A) { body() }
wrap = <A>(x: A) -> A { run(() { use(x) }) }
