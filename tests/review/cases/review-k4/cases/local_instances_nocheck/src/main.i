S(A, N) = (() <- N -> A, (N) -> Nil)
use = <A, N>() <- (s: S(A, N)) {
	S(f, _) = s
	f()
}

main = () {
	run = (body) { body() }
	{
		x = run(() { use() })
		{
			y = run(() { use() })
			Nil
		} <- (S(() { "yes" }, (_: Never) { Nil }))
	} <- (S(() { 7 }, (_: Never) { Nil }))
}
