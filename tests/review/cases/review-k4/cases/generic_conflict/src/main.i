S(A) = (value = A)
use = <A, N>(f: () <- N -> Nil) <- (s: S(A)) {
	f()
	s.value
}
two = () <- (s: S(String)) { Nil }
bad = () -> Int { use(two) }
