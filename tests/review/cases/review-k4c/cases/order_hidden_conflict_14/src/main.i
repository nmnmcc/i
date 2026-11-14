run = <N>(body : () <- N -> Nil) { body() }
one = () <- (s : S(Int)) { Nil }
S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s : S(String)) { Nil }
