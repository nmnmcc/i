Dep = (value = Int)

get = () <- (d : Dep) { d.value }

S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(svc : S(N)) {
	_syn3_0 : S(N) = svc
	use() <- (_syn3_0)
}

main = () {
	assert(wrap(S(f = () { get() }, m = (_ : Dep) { Nil })) == 7) <- (
		Dep(value = 7),
	)
}
