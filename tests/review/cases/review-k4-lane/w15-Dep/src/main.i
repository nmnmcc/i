Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(svc: S(N)) { use() <- (svc) }

main = () {
	assert(wrap(S(f = () { get() }, m = (_: Dep) { Nil })) == 7) <- (
		Dep(value = 7),
	)
}
