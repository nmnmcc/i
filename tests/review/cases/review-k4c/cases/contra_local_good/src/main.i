S(A) = (f = (A) -> Int)
use = <A>(x : A) <- (s : S(A)) { s.f(x) }
wrap = <A>(x : A) {
	local = () { use(x) }
	local()
}

main = () {
	_syn3_0 : S(Int) = S(f = (x : Int) { x + 1 })
	{
		wrap(1)
		Nil
	} <- (_syn3_0)
}
