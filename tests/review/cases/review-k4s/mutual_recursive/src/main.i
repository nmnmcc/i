S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
first: <N>() <- (Fail(Never), ..N) -> Nil = () <- (s: S(N)) -> Nil { second() }
second: <N>() <- (Fail(Never), ..N) -> Nil = () <- (s: S(N)) -> Nil {
	first()
	use()
}
