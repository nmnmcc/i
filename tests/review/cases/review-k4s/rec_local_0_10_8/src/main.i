S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
bad : <N>() <- (Fail(Never), ..N) -> Nil = () -> Nil {
	f : () <- (S(N), ..N) -> Nil = () { use() }
	f()
}
