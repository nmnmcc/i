Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(A, B) = (f = () <- A -> (), g = () <- B -> ())
use = <A, B>() <- (s : S(A, B)) {
	s.f()
	s.g()
}
helper = <A, B>() {
	_syn3_0 : S(Dep, Dep) = S(f = () { get() }, g = () { get() })
	run(() { use() }) <- (_syn3_0)
}

main = () { helper() }
