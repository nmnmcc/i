S(A, N) = (() <- N -> A, (N) -> Nil)
use = <A, N>() <- (s : S(A, N)) {
	S(f, _) = s
	f()
}
run = (body) { body() }

main = () {
	_syn3_0 : S(Int, Never) = S(() { 7 }, (_ : Never) { Nil })

	{
		x = run(() { use() })
		_syn3_1 : S(String, Never) = S(() { "yes" }, (_ : Never) { Nil })
		{
			y = run(() { use() })
			assert(x == 7)
			assert(y == "yes")
		} <- (_syn3_1)
	} <- (_syn3_0)
}
