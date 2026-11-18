S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body : () <- N -> ()) { body() }
one = () <- (s : S(Int)) { () }
two = () <- (s : S(String)) { () }
