S(A) = (A)
run = <N>(body : () <- N -> ()) { body() }
one = () <- (s : S(Int)) { () }
two = () <- (s : S(String)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
