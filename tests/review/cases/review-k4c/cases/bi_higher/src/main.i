S(A) = ()
use = <A>() <- (s: S(A)) { Nil }
wrap = <A>(x: A, f) {
	local = () { f(() { use() }) }
	local()
}
