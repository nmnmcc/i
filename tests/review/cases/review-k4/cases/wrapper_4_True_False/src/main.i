S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = (f) { f() }
id = (x) { x }
f = () -> () {
	f = () { use() }
	f()
}

main = () {
	_syn3_0 : S(Never) = S(() { () }, (_ : Never) { () })
	f() <- (_syn3_0)
}
