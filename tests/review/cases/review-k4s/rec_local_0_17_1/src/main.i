S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
bad : <N>() <- (Fail(Never), S(N), ..N) -> Nil = () -> Nil {
	f : () <- N -> Nil = () { use() }
	f()
}
