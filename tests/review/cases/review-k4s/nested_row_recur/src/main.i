S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
bad : <N>() <- (Fail(Never), S(N)) -> () = () <- (s : S(N)) -> () {
	S(f, _) = s
	f()
}
