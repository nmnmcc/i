one = () <- (s: S(Int)) { Nil }
two = () <- (s: S(String)) { Nil }
S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body: () <- N -> Nil) { body() }
