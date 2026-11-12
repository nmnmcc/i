Dep = (Int)
get = () <- (d: Dep) {
	Dep(x) = d
	x
}
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
make = <N>(f: () <- N -> Int) {
	inner = (g) {
		local = () { g(f) }
		local()
	}
	inner
}
run = <N>(f: () <- N -> Int) { f() }

main = () {
	{
		wrapped = make(() { use() })
		assert(wrapped(run) == 11)
		Nil
	} <- (Dep(11), S(() { get() }))
}
