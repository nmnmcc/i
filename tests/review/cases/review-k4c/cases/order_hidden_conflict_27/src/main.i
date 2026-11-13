one = () <- (s: S(Int)) { Nil }
run = <N>(body: () <- N -> Nil) { body() }
two = () <- (s: S(String)) { Nil }
S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
