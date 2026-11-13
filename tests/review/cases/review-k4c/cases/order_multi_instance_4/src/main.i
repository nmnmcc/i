wrap = <A, N>(x: A) {
	local = () { use(x) }
	run(local)
}
use = <A, N>(x: A) <- (s: S(A, N)) {
	S(f) = s
	f(x)
}
Dep = ()
run = <A, N>(body: () <- N -> A) { body() }
ints = () {
	_syn3_0: S(Int, Never) = S((x: Int) { x + 1 })
	wrap(1) <- (_syn3_0)
}
texts = () {
	_syn3_1: S(String, Never) = S((x: String) { x <> "!" })
	wrap("a") <- (_syn3_1)
}
S(A, N) = ((A) <- N -> A)
