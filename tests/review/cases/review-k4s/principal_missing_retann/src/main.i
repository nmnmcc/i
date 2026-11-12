S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(f: () <- N -> A) { f() }
forward = <N>() <- (s: S(N)) -> Nil {
	local: () <- (S(N), ..N) -> Nil = () { use() }
	run(local)
}
