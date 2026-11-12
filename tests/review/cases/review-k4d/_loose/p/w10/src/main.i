S(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}
wrap = <N>(body: () <- (S(N), ..N) -> Int) {
	body() <- (S(f = () { 1 }, m = (_: N) { Nil }))
}
