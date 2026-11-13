use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
a = <N>() { use() <- (S(() { b() }, (_: N) { Nil })) }
b = () { a() }
S(N) = (() <- N -> Nil, (N) -> Nil)
