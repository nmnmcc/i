RowPair(A, B) = A | B
Dep = ()
get = () <- (d : Dep) { () }
run = <N>(body : () <- N -> ()) { body() }
S(A, B) = (f = () <- A -> (), g = () <- B -> ())
use = <A, B>() <- (s : S(A, B)) {
	s.f()
	s.g()
}

main = <A, B>() {
	local2 = () {
		local : () <- (Fail(Never), RowPair(A, B)) -> () = () { use() }
		local()
	}
	run(local2)
}
