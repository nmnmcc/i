two = () <- (s : S(String)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
S(A) = (A)
one = () <- (s : S(Int)) { () }
run = <N>(body : () <- N -> ()) { body() }
