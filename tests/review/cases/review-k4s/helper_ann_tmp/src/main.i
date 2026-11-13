S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
D = ()
run: <N>(() <- N -> Nil) <- N -> Nil = (body) { body() }
forward = <N>() <- (s: S(N)) -> Nil {
	local = run
	local(() { use() })
}
