S(A) = (A)
one = () <- (s : S(Int)) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
two = () <- (s : S(String)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
