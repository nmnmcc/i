Dep = ()
get = () <- (d : Dep) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
S(N) = (f = () <- N -> Nil)
use = <N>() <- (s : S(N)) { s.f() }

main = <N>() {
	_syn3_0 : S(Dep) = S(f = () { get() })
	{
		local : () <- (S(N), ..N) -> Nil = () { use() }
		local()
	} <- (_syn3_0)
}
