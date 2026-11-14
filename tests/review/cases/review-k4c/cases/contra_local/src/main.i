S(A) = (f = (A) -> Nil)
use = <A>(x : A) <- (s : S(A)) { s.f(x) }
wrap = <A>(x : A) {
	local = () { use(x) }
	local()
}
