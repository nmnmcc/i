Dep = ()
get = () <- (d: Dep) { Nil }
run = <N>(body: () <- N -> Nil) { body() }
S(N) = (f = () <- N -> Nil)
use = <N>() <- (s: S(N)) { s.f() }
helper = <N>() {
	local = () { use() }
	local()
}

main = () { helper() }
