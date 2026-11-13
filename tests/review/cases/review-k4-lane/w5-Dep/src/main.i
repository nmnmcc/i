Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = (body, svc) {
	_syn3_0: S(_) = svc
	body() <- (_syn3_0)
}

main = () {
	assert(wrap(() { use() }, S(f = () { get() })) == 7) <- (Dep(value = 7))
}
