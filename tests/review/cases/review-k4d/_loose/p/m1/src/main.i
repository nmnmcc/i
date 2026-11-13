Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> Nil)
T(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) { s.f() }
uset = <N>() <- (s: T(N)) { s.f() }

main = () {
	{
		uset()
		Nil
	} <- (
		S(f = () { get() }, m = (_: Dep) { Nil }),
		T(f = () { use() }, m = (_: S(Dep)) { Nil }),
	)
}
