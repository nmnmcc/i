one = () <- (s: S(Int)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s: S(String)) { Nil }
S(A) = (A)
run = <N>(body: () <- N -> Nil) { body() }
