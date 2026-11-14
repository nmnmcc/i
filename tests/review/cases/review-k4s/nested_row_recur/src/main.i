S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
bad : <N>() <- (Fail(Never), S(N)) -> Nil = () <- (s : S(N)) -> Nil {
	S(f, _) = s
	f()
}
