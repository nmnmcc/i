S(A, N) = (() <- N -> A, (N) -> Nil)
use = <A, N>() <- (s: S(A, N)) {
	S(f, _) = s
	f()
}
run = (body) { body() }

main = () {
	{
		x = run(() { use() })
		{
			y = run(() { use() })
			assert(x == 7)
			assert(y == "yes")
		} <- (S(() { "yes" }, (_: Never) { Nil }))
	} <- (S(() { 7 }, (_: Never) { Nil }))
}
