Dep = (v = Int)
get = () <- (d : Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> Nil)
T(N) = (f = () <- N -> Int)
use = <N>() <- (s : S(N)) { s.f() }
uset = <N>() <- (s : T(N)) { s.f() }

main = () {
	_syn3_0 : S(Dep) = S(f = () { get() }, m = (_ : Dep) { Nil })
	_syn3_1 : T(S(Dep)) = T(f = () { use() })
	{
		uset()
		Nil
	} <- (_syn3_0, _syn3_1)
}
