S(A) = (A)
one = () <- (s : S(Int)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body : () <- N -> ()) { body() }
two = () <- (s : S(String)) { () }
