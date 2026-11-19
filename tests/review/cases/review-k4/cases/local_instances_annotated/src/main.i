S(A, N) = (() <- N -> A, (N) -> ())
use = <A, N>() <- (s : S(A, N)) {
	S(f, _) = s
	f()
}

main = () {
	run = (body) { body() }
	_syn3_0 : S(Int, Never) = S(() -> Int { 7 }, (_ : Never) { () })
	{
		x : Int = run(() { use() })
		_syn3_1 : S(String, Never) = S(
			() -> String { "yes" },
			(_ : Never) { () },
		)
		{
			y : String = run(() { use() })
			assert(x == 7)
			assert(y == "yes")
		} <- (_syn3_1)
	} <- (_syn3_0)
}
