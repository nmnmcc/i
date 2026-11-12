S(A) = (f = (A) -> Int)
use = <A>(x: A) <- (s: S(A)) { s.f(x) }
wrap = <A>(x: A) {
	local = () { use(x) }
	local()
}
diff: <A, B>(A) <- (S(B)) -> Int = wrap
