two = () <- (s: S(String)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
S(A) = (A)
one = () <- (s: S(Int)) { Nil }
run = <N>(body: () <- N -> Nil) { body() }
