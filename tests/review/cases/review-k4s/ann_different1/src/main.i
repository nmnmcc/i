S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(body : () <- N -> A) { body() }
D = ()
test = <N, M>(f : () <- N -> Nil, g : () <- M -> Nil) -> Nil {
	f()
	g()
}
