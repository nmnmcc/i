two = () <- (s : S(String)) { () }
S(A) = (A)
run = <N>(body : () <- N -> ()) { body() }
bad = () {
	run(() { one() })
	run(() { two() })
}
one = () <- (s : S(Int)) { () }
