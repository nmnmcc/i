one = () <- (s : S(Int)) { () }
run = <N>(body : () <- N -> ()) { body() }
two = () <- (s : S(String)) { () }
S(A) = (A)
bad = () {
	run(() { one() })
	run(() { two() })
}
