bad = () {
	run(() { one() })
	run(() { two() })
}
S(A) = (A)
one = () <- (s : S(Int)) { Nil }
two = () <- (s : S(String)) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
