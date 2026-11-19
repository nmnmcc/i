S(N) = (f = () <- N -> ())
use = <N>() <- (s : S(N)) -> () { s.f() }
run = <N>(body : () <- N -> ()) { body() }
helper = <N>() {
	local : () <- (S(N), ..N) -> () = () { run(() { use() }) }
	run(local)
}
