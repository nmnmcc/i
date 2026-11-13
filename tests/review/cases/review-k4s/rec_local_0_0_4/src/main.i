S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
bad = <N>() {
	f: () <- (S(N)) -> Nil = () { use() }
	f()
}
