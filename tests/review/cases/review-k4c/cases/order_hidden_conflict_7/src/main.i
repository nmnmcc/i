two = () <- (s : S(String)) { () }
run = <N>(body : () <- N -> ()) { body() }
one = () <- (s : S(Int)) { () }
S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
