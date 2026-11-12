S(N) = (() <- N -> Nil, (N) -> Nil)
T(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N), t: T(N)) {
	S(f, _) = s
	T(g, _) = t
	f()
	g()
}
