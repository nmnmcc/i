S(N) = (f = () <- N -> Int, m = (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}
wrap = <N>(body : () <- (S(N), ..N) -> Int) {
	_syn3_0 : S(N) = S(f = () { 1 }, m = (_ : N) { () })
	body() <- (_syn3_0)
}
