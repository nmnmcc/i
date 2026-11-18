S(A) = (value = A)
use = <A, N>(f : () <- N -> ()) <- (s : S(A)) {
	f()
	s.value
}
two = () <- (s : S(String)) { () }
bad : () <- (Fail(Never), S(Int)) -> Int = () <- (fail : Fail(Never)) -> Int {
	use(two)
}
