S(A, N) = (() <- N -> A, (N) -> Nil)
use = <A, N>() <- (s : S(A, N)) {
	S(f, _) = s
	f()
}

main = () {
	run = (body) { body() }
	_syn3_0 : S(Int, Never) = S(() -> Int { 7 }, (_ : Never) { Nil })
	{
		x : Int = run(() { use() })
		_syn3_1 : S(String, Never) = S(
			() -> String { "yes" },
			(_ : Never) { Nil },
		)
		{
			y : String = run(() { use() })
			assert(x == 7)
			assert(y == "yes")
		} <- (_syn3_1)
	} <- (_syn3_0)
}
