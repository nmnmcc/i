S(A, N) = (() <- N -> A, (N) -> ())
use = <A, N>() <- (s : S(A, N)) {
	S(f, _) = s
	f()
}

main = () {
	run = (body) { body() }
	_syn3_0 : S(Int, Never) = S(() { 7 }, (_ : Never) { () })
	{
		x = run(() { use() })
		_syn3_1 : S(String, Never) = S(() { "yes" }, (_ : Never) { () })
		{
			y = run(() { use() })
			assert(x == 7)
			assert(y == "yes")
		} <- (_syn3_1)
	} <- (_syn3_0)
}
