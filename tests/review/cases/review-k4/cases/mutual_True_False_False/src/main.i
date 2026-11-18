S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(body, _) = s
	body()
}
f = <N>(n : Int) <- (s : S(N)) -> () { n |> (0) { use() } & (_) { g(n - 1) } }
g = <M>(n : Int) <- (s : S(M)) { n |> (0) { use() } & (_) { f(n - 1) } }

main = () {
	_syn3_0 : S(Never) = S(() { () }, (_ : Never) { () })
	f(3) <- (_syn3_0)
}
