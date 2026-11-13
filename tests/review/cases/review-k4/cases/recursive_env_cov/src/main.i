S(N) = (() <- N -> Nil)
bad = <N>() <- (s: S(N)) -> Nil {
	S(f) = s
	f()
}
