Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}

wrap: <N>(() <- N -> Int, S(Dep)) <- (Fail(Never), ..N) -> Int = (
	body,
	svc
) <- (fail: Fail(Never)) -> Int {
	_syn3_0: S(Dep) = svc
	body() + get() <- (_syn3_0)
}

main = () {
	_syn3_1: S(Never) = S(f = () { 1 }, m = (_: Never) { Nil })
	{
		wrap(() { use() }, S(f = () { get() }, m = (_: Dep) { Nil }))
		Nil
	} <- (_syn3_1)
}
