Dep = ()
get = () <- (d : Dep) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
S(A, B) = (f = () <- A -> Nil, g = () <- B -> Nil)
use = <A, B>() <- (s : S(A, B)) {
	s.f()
	s.g()
}

main = <A, B>() {
	local2 = () { run(() { use() }) }
	run(local2)
}
