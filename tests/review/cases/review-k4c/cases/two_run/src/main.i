D = (Int)
read = () <- (d: D) {
	D(n) = d
	n
}
S(A, B) = (() <- A -> Int, () <- B -> Int)
use = <A, B>() <- (s: S(A, B)) {
	S(f, g) = s
	f() + g()
}
run = <N>(f: () <- N -> Int) { f() }
wrap = <A, B>() {
	local: () <- (S(A, B), ..A) -> Int = () { run(() { use() }) }
	run(local)
}

main = () {
	_syn3_0: S(D, Never) = S(() { read() }, () { 3 })
	{
		assert(wrap() == 10)
		Nil
	} <- (D(7), _syn3_0)
}
