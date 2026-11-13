b = () { a() }
a = <N>() { use() <- (S(() { b() }, (_: N) { Nil })) }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
S(N) = (() <- N -> Nil, (N) -> Nil)
