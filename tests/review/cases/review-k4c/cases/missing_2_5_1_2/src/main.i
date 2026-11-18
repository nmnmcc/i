Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(E, N) = (f = () <- (Fail(E), ..N) -> ())
use = <E, N>() <- (s : S(E, N)) { s.f() }

main = <E, N>() {
	local2 = () {
		_syn3_0 : S(Never, Dep) = S(f = () { get() })
		run(() -> () { use() }) <- (_syn3_0)
	}
	run(local2)
}
