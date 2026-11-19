bad = () {
	run(() { one() })
	run(() { two() })
}
two = () <- (s : S(String)) { () }
one = () <- (s : S(Int)) { () }
S(A) = (A)
run = <N>(body : () <- N -> ()) { body() }
