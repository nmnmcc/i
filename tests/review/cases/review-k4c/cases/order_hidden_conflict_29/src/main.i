run = <N>(body : () <- N -> ()) { body() }
two = () <- (s : S(String)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
one = () <- (s : S(Int)) { () }
S(A) = (A)
