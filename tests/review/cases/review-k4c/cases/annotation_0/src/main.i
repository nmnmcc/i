S(A) = (f = (A) -> Int)
use = <A>(x: A) <- (s: S(A)) { s.f(x) }
wrap = <A>(x: A) {
	local = () { use(x) }
	local()
}
same: <A>(A) <- (S(A)) -> Int = wrap
