S(N) = (() <- N -> Nil, (N) -> Nil)
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
				Nil
			},
			(_ : Dep) { Nil },
		)
		use() <- (Dep(value = n), _syn3_0)
	}
}
example = () {
	f = make(7)
	f()
}
