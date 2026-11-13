Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(N) = (f = () <- N -> Int, m = (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(body: () <- (S(N), ..N) -> Int, svc: S(N)) { body() <- (svc) }

main = () {
	assert(
		wrap(() { use() }, S(f = () { get() }, m = (_: Dep) { Nil })) == 7,
	) <- (Dep(value = 7))
}
