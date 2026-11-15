S(N) = (() <- N -> Nil, (N) -> Nil)
T(N) = (() <- N -> Nil, (N) -> Nil)
use = <N, M>() <- (s : S(N), t : T(M)) {
	S(f, _) = s
	T(g, _) = t
	f()
	g()
}
