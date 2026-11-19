S(A) = (A)
one = () <- (s : S(Int)) { () }
two = () <- (s : S(String)) { () }
run = <N>(body : () <- N -> ()) { body() }
bad = () {
	run(() { one() })
	run(() { two() })
}
