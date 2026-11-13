S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
first = <N>() <- (s: S(N)) -> Nil { second() }
second = <N>() <- (s: S(N)) -> Nil {
	first()
	use()
}
