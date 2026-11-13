S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
bad = <N>() <- (s: S(N)) {
	f: () <- (S(N), ..N) -> Nil = () { use() }
	f()
}
