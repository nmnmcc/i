S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(body, _) = s
	body()
}
g = <M>(n: Int) <- (s: S(M)) -> Nil { n |> (0) { use() } | (_) { f(n - 1) } }
f = <N>(n: Int) <- (s: S(N)) -> Nil { n |> (0) { use() } | (_) { g(n - 1) } }

main = () { f(3) <- (S(() { Nil }, (_: Never) { Nil })) }
