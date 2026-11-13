S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
make = (f : () -> Nil) {
	() {
		_syn3_0 : S(Never) = S(f, (_ : Never) { Nil })
		use() <- (_syn3_0)
	}
}

main = () {
	f = make(() { Nil })
	f()
}
