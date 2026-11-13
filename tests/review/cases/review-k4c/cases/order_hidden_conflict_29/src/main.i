run = <N>(body: () <- N -> Nil) { body() }
two = () <- (s: S(String)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
one = () <- (s: S(Int)) { Nil }
S(A) = (A)
