S(N) = (() <- N -> Nil)
bad : <N>() <- (Fail(Never), ..N) -> Nil = () <- (s : S(N)) -> Nil {
	S(f) = s
	f()
}
