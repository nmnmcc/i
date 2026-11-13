S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
wrap = <N>() {
	local: () <- (S(N), ..N) -> Nil = () { use() }
	local()
}
