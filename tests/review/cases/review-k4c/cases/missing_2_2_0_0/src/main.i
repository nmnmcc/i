Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(E, N) = (f = () <- (Fail(E), ..N) -> ())
use = <E, N>() <- (s : S(E, N)) { s.f() }

main = <E, N>() {
	local = () { use() }
	local()
}
