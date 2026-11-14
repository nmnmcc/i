S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s : S(String)) { Nil }
one = () <- (s : S(Int)) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
