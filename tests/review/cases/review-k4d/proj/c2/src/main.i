A(N) = (() <- N -> Int)
B(N) = (() <- N -> Int)
ua = <N>() <- (s: A(N)) {
	A(f) = s
	f()
}
ub = <N>() <- (s: B(N)) {
	B(f) = s
	f()
}
bad = () { ua() <- (B(() { ua() }), A(() { ub() + 1 })) }
bad2 = <M>() { ua() <- (B(() { ua() }), A(() { ub() + 1 })) }
grow = (x) { grow([x]) }
self = (x) { x(x) }
