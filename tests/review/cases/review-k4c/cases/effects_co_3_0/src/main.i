S(A, E, N) = ((A) <- (Fail(E), ..N) -> A)
use = <A, E, N>(x : A) <- (s : S(A, E, N)) {
	S(f) = s
	f(x)
}
run = <A, N>(body : () <- N -> A) { body() }
wrap = <A, E, N>(x : A) {
	local : () <- (Fail(E), S(A, E, N), ..N) -> A = () { use(x) }
	local()
}
