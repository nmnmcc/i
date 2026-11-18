S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
wrap = <N>() {
	local : () <- (S(N), ..N) -> () = () { use() }
	local()
}
