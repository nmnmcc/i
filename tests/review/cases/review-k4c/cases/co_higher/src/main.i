S(A) = (value = A)
use = <A>() <- (s: S(A)) { s.value }
wrap = <A>(x: A, f) {
	local = () { f(() { use() }) }
	local()
}
