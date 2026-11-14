S(A) = (f = (A) -> A)
use = <A>(x : A) <- (s : S(A)) { s.f(x) }
wrap = <A>(x : A) {
	local = () {
		inner = () { use(x) }
		inner()
	}
	local()
}
