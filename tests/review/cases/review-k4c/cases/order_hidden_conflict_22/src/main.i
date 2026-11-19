two = () <- (s : S(String)) { () }
one = () <- (s : S(Int)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body : () <- N -> ()) { body() }
S(A) = (A)
