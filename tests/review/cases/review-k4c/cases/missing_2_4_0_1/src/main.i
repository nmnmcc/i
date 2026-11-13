Dep = ()
get = () <- (d: Dep) { Nil }
run = <N>(body: () <- N -> Nil) { body() }
S(E, N) = (f = () <- (Fail(E), ..N) -> Nil)
use = <E, N>() <- (s: S(E, N)) { s.f() }
helper = <E, N>() {
	local: () <- N -> Nil = () { use() }
	local()
}

main = () { helper() }
