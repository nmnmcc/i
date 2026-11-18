one = () <- (s : S(Int)) { () }
run = <N>(body : () <- N -> ()) { body() }
bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s : S(String)) { () }
S(A) = (A)
