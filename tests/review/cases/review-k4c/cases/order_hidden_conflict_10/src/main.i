S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s : S(String)) { () }
one = () <- (s : S(Int)) { () }
run = <N>(body : () <- N -> ()) { body() }
