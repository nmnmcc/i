Dep = ()
S(A, N) = ((A) <- N -> A)
use = <A, N>(x: A) <- (s: S(A, N)) {
	S(f) = s
	f(x)
}
wrap = <A, N>(x: A) {
	local = () { use(x) }
	run(local)
}
run = <A, N>(body: () <- N -> A) { body() }
ints = () { wrap(1) <- (S((x: Int) { x + 1 })) }
texts = () { wrap("a") <- (S((x: String) { x <> "!" })) }
