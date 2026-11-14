two = () <- (s : S(String)) { Nil }
S(A) = (A)
run = <N>(body : () <- N -> Nil) { body() }
bad = () {
	run(() { one() })
	run(() { two() })
}
one = () <- (s : S(Int)) { Nil }
