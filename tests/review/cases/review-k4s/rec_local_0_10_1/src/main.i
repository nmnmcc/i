S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
bad = <N>() -> Nil {
	f : () <- N -> Nil = () { use() }
	f()
}
