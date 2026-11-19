S(A) = (A)
one = () <- (s : S(Int)) { () }
run = <N>(body : () <- N -> ()) { body() }
two = () <- (s : S(String)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
