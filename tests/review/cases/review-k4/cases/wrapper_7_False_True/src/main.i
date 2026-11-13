S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = (f) { f() }
id = (x) { x }
f = <N>() <- (s : S(N)) {
	f = () { use() }
	run(f)
	run(f)
}

main = () {
	_syn3_0 : S(Never) = S(() { Nil }, (_ : Never) { Nil })
	f() <- (_syn3_0)
}
