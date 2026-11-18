Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (f = () <- N -> ())
use = <N>() <- (s : S(N)) { s.f() }
helper = <N>() {
	_syn3_0 : S(Dep) = S(f = () { get() })
	{
		local : () <- (S(N), ..N) -> () = () { run(() { use() }) }
		run(local)
	} <- (_syn3_0)
}

main = () { helper() }
