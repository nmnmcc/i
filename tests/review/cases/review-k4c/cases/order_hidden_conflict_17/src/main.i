bad = () {
	run(() { one() })
	run(() { two() })
}
one = () <- (s : S(Int)) { Nil }
two = () <- (s : S(String)) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
S(A) = (A)
