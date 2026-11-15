S(N) = (() <- N -> Nil)
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
make = <N>(f : () <- N -> Nil) {
	() {
		_syn3_0 : S(N) = S(f)
		use() <- (_syn3_0)
	}
}

main = () {
	f = make(() { Nil })
	f()
}
