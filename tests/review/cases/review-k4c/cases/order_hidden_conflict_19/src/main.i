bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s: S(String)) { Nil }
one = () <- (s: S(Int)) { Nil }
S(A) = (A)
run = <N>(body: () <- N -> Nil) { body() }
