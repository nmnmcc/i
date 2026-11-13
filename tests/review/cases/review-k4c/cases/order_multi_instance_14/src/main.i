S(A, N) = ((A) <- N -> A)
texts = () { wrap("a") <- (S((x: String) { x <> "!" })) }
use = <A, N>(x: A) <- (s: S(A, N)) {
	S(f) = s
	f(x)
}
Dep = ()
wrap = <A, N>(x: A) {
	local = () { use(x) }
	run(local)
}
ints = () { wrap(1) <- (S((x: Int) { x + 1 })) }
run = <A, N>(body: () <- N -> A) { body() }
