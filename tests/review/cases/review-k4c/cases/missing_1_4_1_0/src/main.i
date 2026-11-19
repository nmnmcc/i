Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (f = () <- N -> (), mark = (N) -> ())
use = <N>() <- (s : S(N)) { s.f() }

main = <N>() {
	_syn3_0 : S(Dep) = S(f = () { get() }, mark = (_ : Dep) { () })
	{
		local : () <- N -> () = () { use() }
		local()
	} <- (_syn3_0)
}
