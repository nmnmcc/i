b = () { a() }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
a = <N>() { use() <- (S(() { b() }, (_: N) { Nil })) }
S(N) = (() <- N -> Nil, (N) -> Nil)
