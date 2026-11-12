S(A) = (A)
one = () <- (s: S(Int)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body: () <- N -> Nil) { body() }
two = () <- (s: S(String)) { Nil }
