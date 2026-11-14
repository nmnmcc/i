Dep = ()
get = () <- (d : Dep) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
S(N) = (f = () <- N -> Nil)
use = <N>() <- (s : S(N)) { s.f() }
helper = <N>() {
	_syn3_0 : S(Dep) = S(f = () { get() })
	run(() -> Nil { use() }) <- (_syn3_0)
}

main = () { helper() }
