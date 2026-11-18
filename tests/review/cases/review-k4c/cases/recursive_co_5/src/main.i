S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(f : () <- N -> ()) { f() }
bad : <N>() <- (Fail(Never), ..N) -> () = () <- (s : S(N)) -> () {
	local = () { use() }
	local()
}
