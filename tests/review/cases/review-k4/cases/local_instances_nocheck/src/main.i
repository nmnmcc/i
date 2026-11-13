S(A, N) = (() <- N -> A, (N) -> Nil)
use = <A, N>() <- (s: S(A, N)) {
	S(f, _) = s
	f()
}

main = () {
	run = (body) { body() }
	_syn3_0: S(Int, Never) = S(() { 7 }, (_: Never) { Nil })
	{
		x = run(() { use() })
		_syn3_1: S(String, Never) = S(() { "yes" }, (_: Never) { Nil })
		{
			y = run(() { use() })
			Nil
		} <- (_syn3_1)
	} <- (_syn3_0)
}
