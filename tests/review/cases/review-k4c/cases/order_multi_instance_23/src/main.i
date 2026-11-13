use = <A, N>(x: A) <- (s: S(A, N)) {
	S(f) = s
	f(x)
}
Dep = ()
S(A, N) = ((A) <- N -> A)
wrap = <A, N>(x: A) {
	local = () { use(x) }
	run(local)
}
run = <A, N>(body: () <- N -> A) { body() }
texts = () {
	_syn3_0: S(String, Never) = S((x: String) { x <> "!" })
	wrap("a") <- (_syn3_0)
}
ints = () {
	_syn3_1: S(Int, Never) = S((x: Int) { x + 1 })
	wrap(1) <- (_syn3_1)
}
