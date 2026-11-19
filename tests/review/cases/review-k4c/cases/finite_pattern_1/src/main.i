S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
helper = <N>() {
	local : () <- (S(N), ..N) -> () = () { use() }
	local()
}
