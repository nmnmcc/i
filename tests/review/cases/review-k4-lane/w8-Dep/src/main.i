Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = (svc) {
	_syn3_0: S(_) = svc
	use() <- (_syn3_0)
}

main = () { assert(wrap(S(f = () { get() })) == 7) <- (Dep(value = 7)) }
