bad = () {
	run(() { one() })
	run(() { two() })
}
run = <N>(body : () <- N -> ()) { body() }
S(A) = (A)
two = () <- (s : S(String)) { () }
one = () <- (s : S(Int)) { () }
