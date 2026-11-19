Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (f = () <- N -> (), mark = (N) -> ())
use = <N>() <- (s : S(N)) { s.f() }

main = <N>() {
	local2 = () {
		local = () { use() }
		local()
	}
	run(local2)
}
