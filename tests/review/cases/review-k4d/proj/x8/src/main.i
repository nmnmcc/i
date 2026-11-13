Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

inner = (body, svc: S(Dep)) {
	_syn3_0: S(Dep) = svc
	body() <- (_syn3_0)
}
wrap = <N>(body: () <- N -> Int, svc: S(Dep)) -> Int { inner(body, svc) }

main = () {
	_syn3_1: S(Never) = S(f = () { 1 }, m = (_: Never) { Nil })
	{
		wrap(() { use() }, S(f = () { get() }, m = (_: Dep) { Nil }))
		Nil
	} <- (_syn3_1)
}
