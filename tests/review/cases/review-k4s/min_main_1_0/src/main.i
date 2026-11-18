S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}

main = <N>() <- (s : S(N)) {
	local : () <- (S(N), ..N) -> () = () { use() }
	local()
}
