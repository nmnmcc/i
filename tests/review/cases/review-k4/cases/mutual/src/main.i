S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
f = (n : Int) { n |> (0) { use() } | (_) { g(n - 1) } }
g = (n : Int) { n |> (0) { use() } | (_) { f(n - 1) } }

main = () {
	_syn3_0 : S(Never) = S(() { Nil }, (_ : Never) { Nil })
	f(3) <- (_syn3_0)
}
