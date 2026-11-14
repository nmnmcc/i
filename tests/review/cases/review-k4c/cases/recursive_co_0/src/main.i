S(N) = (() <- N -> Nil)
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(f : () <- N -> Nil) { f() }
bad : <N>() <- (Fail(Never), ..N) -> Nil = () <- (s : S(N)) -> Nil {
	S(f) = s
	f()
}
