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
bad1 = () {
	_syn3_0: A(B(Never)) = A(() { ub() + 1 })
	1 <- (_syn3_0)
}
mk = () -> A(B(A(Never))) { A(() { ub() + 1 }) }
bad2 = () {
	_syn3_1: B(A(Never)) = B(() { ua() })
	_syn3_2: A(B(A(Never))) = A(() { ub() + 1 })
	2 <- (_syn3_1, _syn3_2)
}
bad3 = () {
	_syn3_3: B(A(Never)) = B(() { ua() })
	_syn3_4: A(B(A(Never))) = A(() { ub() + 1 })
	ua() <- (_syn3_3, _syn3_4)
}
