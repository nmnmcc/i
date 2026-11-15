two = () <- (s : S(String)) { Nil }
one = () <- (s : S(Int)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body : () <- N -> Nil) { body() }
S(A) = (A)
