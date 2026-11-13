Dep = ()
get = () <- (d: Dep) { Nil }
run = <N>(body: () <- N -> Nil) { body() }
S(A, B) = (f = () <- A -> Nil, g = () <- B -> Nil)
use = <A, B>() <- (s: S(A, B)) {
	s.f()
	s.g()
}
helper = <A, B>() {
	run(() { use() }) <- (S(f = () { get() }, g = () { get() }))
}

main = () { helper() }
