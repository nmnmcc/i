S(N) = (() <- N -> Int, (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
a = (x : Int) -> Int { x |> (0) { use() } & (_) { b(x - 1) } }
b = (x : Int) -> Int { x |> (0) { use() } & (_) { a(x - 1) } }

main = () {
	_syn3_0 : S(Never) = S(() { 42 }, (_ : Never) { () })
	{
		assert(a(4) == 42)
		assert(b(3) == 42)
		()
	} <- (_syn3_0)
}
