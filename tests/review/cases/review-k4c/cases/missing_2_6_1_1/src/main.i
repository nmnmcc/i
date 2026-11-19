Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(E, N) = (f = () <- (Fail(E), ..N) -> ())
use = <E, N>() <- (s : S(E, N)) { s.f() }
helper = <E, N>() {
	_syn3_0 : S(Never, Dep) = S(f = () { get() })
	{
		local : () <- (Fail(E), S(E, N), ..N) -> () = () { run(() { use() }) }
		run(local)
	} <- (_syn3_0)
}

main = () { helper() }
