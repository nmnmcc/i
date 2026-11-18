Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (f = () <- N -> ())
use = <N>() <- (s : S(N)) { s.f() }

main = <N>() {
	local2 = () {
		local : () <- (S(N), ..N) -> () = () { use() }
		local()
	}
	run(local2)
}
