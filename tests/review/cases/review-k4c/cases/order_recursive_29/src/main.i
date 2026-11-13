S(N) = (() <- N -> Nil, (N) -> Nil)
a = <N>() { use() <- (S(() { b() }, (_: N) { Nil })) }
b = () { a() }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
