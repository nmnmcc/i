S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}
wrap: <N>(() <- (S(N), ..N) -> Int) <- N -> Int = (body) {
	body() <- (S(f = () { 1 }))
}
