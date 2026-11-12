S(A) = ()
use = <A>() <- (s: S(A)) { Nil }
wrap = <A>(x: A) {
	local = () {
		inner = () { use() }
		inner()
	}
	local()
}
