S(A) = (A)
two = () <- (s : S(String)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body : () <- N -> ()) { body() }
one = () <- (s : S(Int)) { () }
