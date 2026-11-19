bad = () {
	run(() { one() })
	run(() { two() })
}
S(A) = (A)
two = () <- (s : S(String)) { () }
run = <N>(body : () <- N -> ()) { body() }
one = () <- (s : S(Int)) { () }
