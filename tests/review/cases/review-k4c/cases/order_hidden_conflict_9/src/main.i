S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body : () <- N -> Nil) { body() }
one = () <- (s : S(Int)) { Nil }
two = () <- (s : S(String)) { Nil }
