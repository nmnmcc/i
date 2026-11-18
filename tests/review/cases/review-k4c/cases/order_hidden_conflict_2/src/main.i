S(A) = (A)
two = () <- (s : S(String)) { () }
one = () <- (s : S(Int)) { () }
run = <N>(body : () <- N -> ()) { body() }
bad = () {
	run(() { one() })
	run(() { two() })
}
