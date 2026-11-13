S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
make = <N>(f: () <- N -> Nil) { use() <- (S(f, (_: N) { Nil })) }

main = () { make(() { Nil }) }
