wrap = <A, N>(x: A) {
	local = () { use(x) }
	run(local)
}
Dep = ()
run = <A, N>(body: () <- N -> A) { body() }
use = <A, N>(x: A) <- (s: S(A, N)) {
	S(f) = s
	f(x)
}
S(A, N) = ((A) <- N -> A)
ints = () { wrap(1) <- (S((x: Int) { x + 1 })) }
texts = () { wrap("a") <- (S((x: String) { x <> "!" })) }
