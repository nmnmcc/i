bad = () {
	run(() { one() })
	run(() { two() })
}
S(A) = (A)
one = () <- (s : S(Int)) { () }
two = () <- (s : S(String)) { () }
run = <N>(body : () <- N -> ()) { body() }
