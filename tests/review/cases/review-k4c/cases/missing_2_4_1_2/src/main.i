Dep = ()
get = () <- (d: Dep) { Nil }
run = <N>(body: () <- N -> Nil) { body() }
S(E, N) = (f = () <- (Fail(E), ..N) -> Nil)
use = <E, N>() <- (s: S(E, N)) { s.f() }

main = <E, N>() {
	local2 = () {
		_syn3_0: S(Never, Dep) = S(f = () { get() })
		{
			local: () <- N -> Nil = () { use() }
			local()
		} <- (_syn3_0)
	}
	run(local2)
}
