Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = <N>(body: () <- N -> Int, svc: S(Dep)) -> Int { body() <- (svc) }

main = () {
	{
		wrap(() { use() }, S(f = () { get() }, m = (_: Dep) { Nil }))
		Nil
	} <- (S(f = () { 1 }, m = (_: Never) { Nil }))
}
