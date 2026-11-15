one = () <- (s : S(Int)) { Nil }
S(A) = (A)
run = <N>(body : () <- N -> Nil) { body() }
two = () <- (s : S(String)) { Nil }
bad = () {
	run(() { one() })
	run(() { two() })
}
