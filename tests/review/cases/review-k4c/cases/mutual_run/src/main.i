S(N) = (() <- N -> Int, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
a = (x: Int) -> Int { x |> (0) { use() } | (_) { b(x - 1) } }
b = (x: Int) -> Int { x |> (0) { use() } | (_) { a(x - 1) } }

main = () {
	{
		assert(a(4) == 42)
		assert(b(3) == 42)
		Nil
	} <- (S(() { 42 }, (_: Never) { Nil }))
}
