bad = () {
	run(() { one() })
	run(() { two() })
}
S(A) = (A)
two = () <- (s : S(String)) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
one = () <- (s : S(Int)) { Nil }
