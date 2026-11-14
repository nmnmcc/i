S(A, E, N, M) = ((A) <- (Fail(E), ..N) -> A, () <- M -> Nil)
use = <A, E, N, M>(x : A) <- (s : S(A, E, N, M)) {
	S(f, g) = s
	g()
	f(x)
}
run = <A, N>(body : () <- N -> A) { body() }
wrap = <A, E, N, M>(x : A) {
	local = () {
		inner = () { use(x) }
		inner()
	}
	run(local)
}
