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
	_syn3_0 : S(Dep, Dep) = S(f = () { get() }, g = () { get() })
	{
		local : () <- (Fail(Never), S(A, B), RowPair(A, B)) -> () = () {
			run(() { use() })
		}
		run(local)
	} <- (_syn3_0)
}
