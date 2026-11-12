Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(N) = (f = () <- N -> Int)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = (body, svc) { body() <- (svc) }

main = () {
	assert(wrap(() { use() }, S(f = () { get() })) == 7) <- (Dep(value = 7))
}
