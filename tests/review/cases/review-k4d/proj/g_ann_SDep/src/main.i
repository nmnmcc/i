Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (() <- N -> Int)
T(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) -> Int {
	S(f) = s
	f()
}
uset = <N>() <- (s: T(N)) {
	T(f) = s
	f()
}

main = () {
	{
		uset()
		Nil
	} <- (S(() { get() }), T(() { use() }))
}
