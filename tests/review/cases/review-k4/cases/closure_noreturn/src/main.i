S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
make = <N>(f : () <- N -> ()) {
	_syn3_0 : S(N) = S(f, (_ : N) { () })
	use() <- (_syn3_0)
}

main = () { make(() { () }) }
