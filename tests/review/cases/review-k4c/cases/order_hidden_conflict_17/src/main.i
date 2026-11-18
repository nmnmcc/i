bad = () {
	run(() { one() })
	run(() { two() })
}
one = () <- (s : S(Int)) { () }
two = () <- (s : S(String)) { () }
run = <N>(body : () <- N -> ()) { body() }
S(A) = (A)
