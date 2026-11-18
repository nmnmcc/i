two = () <- (s : S(String)) { () }
run = <N>(body : () <- N -> ()) { body() }
bad = () {
	run(() { one() })
	run(() { two() })
}
S(A) = (A)
one = () <- (s : S(Int)) { () }
