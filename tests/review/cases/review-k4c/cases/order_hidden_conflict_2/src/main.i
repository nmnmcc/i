S(A) = (A)
two = () <- (s : S(String)) { Nil }
one = () <- (s : S(Int)) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
bad = () {
	run(() { one() })
	run(() { two() })
}
