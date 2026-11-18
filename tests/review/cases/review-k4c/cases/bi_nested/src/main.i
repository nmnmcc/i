S(A) = ()
use = <A>() <- (s : S(A)) { () }
wrap = <A>(x : A, f) {
	local = () { f() }
	use()
	local()
}
