S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}

main = <N>() {
	local: () <- (S(N), ..N) -> Nil = () { use() }
	local()
}
