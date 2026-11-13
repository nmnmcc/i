S(A) = (A)
two = () <- (s: S(String)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body: () <- N -> Nil) { body() }
one = () <- (s: S(Int)) { Nil }
