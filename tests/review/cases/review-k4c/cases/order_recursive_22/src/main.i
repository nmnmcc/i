a = <N>() { use() <- (S(() { b() }, (_: N) { Nil })) }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
b = () { a() }
S(N) = (() <- N -> Nil, (N) -> Nil)
