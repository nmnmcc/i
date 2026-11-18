S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
Dep = (value = Int)
get = () <- (d : Dep) { d.value }
make = (n : Int) {
	() {
		_syn3_0 : S(Dep) = S(
			() {
				get()
				()
			},
			(_ : Dep) { () },
		)
		use() <- (Dep(value = n), _syn3_0)
	}
}
example = () {
	f = make(7)
	f()
}
