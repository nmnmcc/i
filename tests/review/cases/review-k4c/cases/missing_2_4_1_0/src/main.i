Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(E, N) = (f = () <- (Fail(E), ..N) -> ())
use = <E, N>() <- (s : S(E, N)) { s.f() }

main = <E, N>() {
	_syn3_0 : S(Never, Dep) = S(f = () { get() })
	{
		local : () <- N -> () = () { use() }
		local()
	} <- (_syn3_0)
}
