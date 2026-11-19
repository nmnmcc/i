Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(A, B) = (f = () <- A -> (), g = () <- B -> ())
use = <A, B>() <- (s : S(A, B)) {
	s.f()
	s.g()
}
helper = <A, B>() {
	local = () {
		inner = () { use() }
		inner()
	}
	run(local)
}

main = () { helper() }
