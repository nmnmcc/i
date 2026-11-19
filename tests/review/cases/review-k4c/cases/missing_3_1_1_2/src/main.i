Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(A, B) = (f = () <- A -> (), g = () <- B -> ())
use = <A, B>() <- (s : S(A, B)) {
	s.f()
	s.g()
}

main = <A, B>() {
	local2 = () {
		_syn3_0 : S(Dep, Dep) = S(f = () { get() }, g = () { get() })
		run(() { use() }) <- (_syn3_0)
	}
	run(local2)
}
