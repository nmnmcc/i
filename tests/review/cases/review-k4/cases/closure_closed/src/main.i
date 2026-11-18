S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
make = (f : () -> ()) {
	() {
		_syn3_0 : S(Never) = S(f, (_ : Never) { () })
		use() <- (_syn3_0)
	}
}

main = () {
	f = make(() { () })
	f()
}
