Dep = ()
get = () <- (d : Dep) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
S(N) = (f = () <- N -> Nil, mark = (N) -> Nil)
use = <N>() <- (s : S(N)) { s.f() }

main = <N>() {
	local = () { use() }
	local()
}
