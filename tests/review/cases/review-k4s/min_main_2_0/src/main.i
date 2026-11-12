S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}

main = <N>() <- (s: S(N)) {
	local = () -> Nil { use() }
	local()
}
