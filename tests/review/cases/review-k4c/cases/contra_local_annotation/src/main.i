S(A) = (f = (A) -> Int)
use = <A>(x: A) <- (s: S(A)) { s.f(x) }
wrap = <A>(x: A) {
	local: () <- (S(A)) -> Int = () { use(x) }
	local()
}

main = () {
	{
		wrap(1)
		Nil
	} <- (S(f = (_: String) { 42 }))
}
