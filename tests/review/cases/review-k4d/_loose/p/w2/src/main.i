S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}
wrap = <N>(body: () <- (S(N), ..N) -> Int) { body() <- (S(f = () { 1 })) }
