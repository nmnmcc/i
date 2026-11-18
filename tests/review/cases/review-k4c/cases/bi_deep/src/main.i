S(A) = ()
use = <A>() <- (s : S(A)) { () }
wrap = <A>(x : A) {
	local = () {
		inner = () { use() }
		inner()
	}
	local()
}
