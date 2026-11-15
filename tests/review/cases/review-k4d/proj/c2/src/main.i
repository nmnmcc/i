A(N) = (() <- N -> Int)
B(N) = (() <- N -> Int)
ua = <N>() <- (s : A(N)) {
	A(f) = s
	f()
}
ub = <N>() <- (s : B(N)) {
	B(f) = s
	f()
}
bad = () {
	_syn3_0 : B(A(Never)) = B(() { ua() })
	_syn3_1 : A(B(A(Never))) = A(() { ub() + 1 })
	ua() <- (_syn3_0, _syn3_1)
}
bad2 = <M>() {
	_syn3_2 : B(M) = B(() { ua() })
	_syn3_3 : A(M) = A(() { ub() + 1 })
	ua() <- (_syn3_2, _syn3_3)
}
grow = (x) { grow([x]) }
self = (x) { x(x) }
