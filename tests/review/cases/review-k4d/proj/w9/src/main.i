S(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}
wrap: <N>(() <- (S(N), ..N) -> Int) <- N -> Int = (body) {
	_syn3_0: S(N) = S(f = () { 1 }, m = (_: N) { Nil })
	body() <- (_syn3_0)
}
