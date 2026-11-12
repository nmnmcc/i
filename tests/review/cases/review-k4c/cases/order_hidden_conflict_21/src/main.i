S(A) = (A)
run = <N>(body: () <- N -> Nil) { body() }
one = () <- (s: S(Int)) { Nil }
two = () <- (s: S(String)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
