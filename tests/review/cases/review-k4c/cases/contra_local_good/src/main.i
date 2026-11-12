S(A) = (f = (A) -> Int)
use = <A>(x: A) <- (s: S(A)) { s.f(x) }
wrap = <A>(x: A) {
	local = () { use(x) }
	local()
}

main = () {
	{
		wrap(1)
		Nil
	} <- (S(f = (x: Int) { x + 1 }))
}
