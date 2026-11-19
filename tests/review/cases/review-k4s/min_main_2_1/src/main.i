S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}

main = <N>() <- (s : S(N)) {
	local = () -> () { use() }
	local()
}
