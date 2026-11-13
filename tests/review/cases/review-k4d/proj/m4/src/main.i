Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int)
T(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) { s.f() }
uset = <N>() <- (s: T(N)) { s.f() }

main = () {
	_syn3_0: S(Dep) = S(f = () { get() })
	_syn3_1: T(S(Dep)) = T(f = () { use() }, m = (_) { Nil })
	{
		uset()
		Nil
	} <- (_syn3_0, _syn3_1)
}
