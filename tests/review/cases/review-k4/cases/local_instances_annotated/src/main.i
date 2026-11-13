S(A, N) = (() <- N -> A, (N) -> Nil)
use = <A, N>() <- (s: S(A, N)) {
	S(f, _) = s
	f()
}

main = () {
	run = (body) { body() }
	{
		x: Int = run(() { use() })
		{
			y: String = run(() { use() })
			assert(x == 7)
			assert(y == "yes")
		} <- (S(() { "yes" }, (_: Never) { Nil }))
	} <- (S(() { 7 }, (_: Never) { Nil }))
}
