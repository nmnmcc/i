Dep = ()
get = () <- (d: Dep) { Nil }
run = <N>(body: () <- N -> Nil) { body() }
S(E, N) = (f = () <- (Fail(E), ..N) -> Nil)
use = <E, N>() <- (s: S(E, N)) { s.f() }
helper = <E, N>() {
	_syn3_0: S(Never, Dep) = S(f = () { get() })
	{
		local: () <- (Fail(E), S(E, N), ..N) -> Nil = () { run(() { use() }) }
		run(local)
	} <- (_syn3_0)
}

main = () { helper() }
