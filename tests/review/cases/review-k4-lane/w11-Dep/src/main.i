Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap: <N>(() <- (S(N), ..N) -> Int, S(N)) <- N -> Int = (body, svc) {
	_syn3_0: S(N) = svc
	body() <- (_syn3_0)
}

main = () {
	assert(
		wrap(() { use() }, S(f = () { get() }, m = (_: Dep) { Nil })) == 7,
	) <- (Dep(value = 7))
}
