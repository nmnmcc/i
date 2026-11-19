run = <N>(body : () <- N -> ()) { body() }
S(A) = (A)
two = () <- (s : S(String)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
one = () <- (s : S(Int)) { () }
