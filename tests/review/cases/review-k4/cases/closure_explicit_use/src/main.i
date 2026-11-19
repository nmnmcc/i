S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
make = <N>(f : () <- N -> ()) {
	() {
		_syn3_0 : S(N) = S(f, (_ : N) { () })
		{
			use_n : () <- (S(N), ..N) -> () = use
			use_n()
		} <- (_syn3_0)
	}
}

main = () {
	f = make(() { () })
	f()
}
