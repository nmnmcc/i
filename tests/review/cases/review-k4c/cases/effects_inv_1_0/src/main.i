S(A, E, N) = ((A) <- (Fail(E), ..N) -> A, (N) -> Nil)
use = <A, E, N>(x: A) <- (s: S(A, E, N)) {
	S(f, _) = s
	f(x)
}
run = <A, N>(body: () <- N -> A) { body() }
wrap = <A, E, N>(x: A) { run(() { use(x) }) }
