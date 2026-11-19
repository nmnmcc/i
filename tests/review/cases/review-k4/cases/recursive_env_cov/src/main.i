S(N) = (() <- N -> ())
bad : <N>() <- (Fail(Never), ..N) -> () = () <- (s : S(N)) -> () {
	S(f) = s
	f()
}
