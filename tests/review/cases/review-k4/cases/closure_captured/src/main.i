S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
make = <N>(s : S(N)) {
	() {
		_syn3_0 : S(N) = s
		use() <- (_syn3_0)
	}
}

main = () {
	f = make(S(() { () }, (_ : Never) { () }))
	f()
}
