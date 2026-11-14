S(A) = (f = () <- A -> Nil)
use = <A>() <- (s : S(A)) { s.f() }
wrap = <A>(x : A) {
	local = () {
		inner = () { use() }
		inner()
	}
	local()
}
