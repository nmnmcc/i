S(A, N) = ((A) <- N -> A)
texts = () { wrap("a") <- (S((x: String) { x <> "!" })) }
ints = () { wrap(1) <- (S((x: Int) { x + 1 })) }
wrap = <A, N>(x: A) {
	local = () { use(x) }
	run(local)
}
use = <A, N>(x: A) <- (s: S(A, N)) {
	S(f) = s
	f(x)
}
run = <A, N>(body: () <- N -> A) { body() }
Dep = ()
