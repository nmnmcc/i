one = () <- (s: S(Int)) { Nil }
run = <N>(body: () <- N -> Nil) { body() }
bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s: S(String)) { Nil }
S(A) = (A)
