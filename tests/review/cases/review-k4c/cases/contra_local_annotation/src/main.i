S(A) = (f = (A) -> Int)
use = <A>(x : A) <- (s : S(A)) { s.f(x) }
wrap = <A>(x : A) {
	local : () <- (S(A)) -> Int = () { use(x) }
	local()
}

main = () {
	_syn3_0 : S(String) = S(f = (_ : String) { 42 })
	{
		wrap(1)
		Nil
	} <- (_syn3_0)
}
