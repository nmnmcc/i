use = <A, N>(x: A) <- (s: S(A, N)) {
	S(f) = s
	f(x)
}
ints = () { wrap(1) <- (S((x: Int) { x + 1 })) }
S(A, N) = ((A) <- N -> A)
run = <A, N>(body: () <- N -> A) { body() }
texts = () { wrap("a") <- (S((x: String) { x <> "!" })) }
wrap = <A, N>(x: A) {
	local = () { use(x) }
	run(local)
}
Dep = ()
