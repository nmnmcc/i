Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (f = () <- N -> ())
use = <N>() <- (s : S(N)) { s.f() }

main = <N>() {
	local2 = () {
		local : () <- (S(N), ..N) -> () = () { run(() { use() }) }
		run(local)
	}
	run(local2)
}
