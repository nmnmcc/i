S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
first : <N>() <- (Fail(Never), ..N) -> () = () <- (s : S(N)) -> () {
	second()
}
second : <N>() <- (Fail(Never), ..N) -> () = () <- (s : S(N)) -> () {
	first()
	use()
}
