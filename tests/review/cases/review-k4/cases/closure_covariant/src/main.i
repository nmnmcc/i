S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
make = <N>(f : () <- N -> ()) {
	() {
		_syn3_0 : S(N) = S(f)
		use() <- (_syn3_0)
	}
}

main = () {
	f = make(() { () })
	f()
}
