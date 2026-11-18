one = () <- (s : S(Int)) { () }
bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s : S(String)) { () }
S(A) = (A)
run = <N>(body : () <- N -> ()) { body() }
