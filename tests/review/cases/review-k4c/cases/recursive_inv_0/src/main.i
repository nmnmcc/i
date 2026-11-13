S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <N>(f: () <- N -> Nil) { f() }
bad: <N>() <- (Fail(Never), ..N) -> Nil = () <- (s: S(N)) -> Nil {
	S(f, _) = s
	f()
}
