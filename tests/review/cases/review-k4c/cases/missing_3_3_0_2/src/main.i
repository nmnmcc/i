RowPair(A, B) = A | B
Dep = ()
get = () <- (d : Dep) { Nil }
run = <N>(body : () <- N -> Nil) { body() }
S(A, B) = (f = () <- A -> Nil, g = () <- B -> Nil)
use = <A, B>() <- (s : S(A, B)) {
	s.f()
	s.g()
}

main = <A, B>() {
	local2 = () {
		local : () <- (Fail(Never), S(A, B), RowPair(A, B)) -> Nil = () {
			use()
		}
		local()
	}
	run(local2)
}
