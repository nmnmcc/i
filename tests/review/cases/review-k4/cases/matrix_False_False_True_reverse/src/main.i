S(N) = (() <- N -> ())
f = <N>() <- (s : S(N)) -> () { run(() { use() }) }
run = (body) { body() }
use = <N>() <- (s : S(N)) -> () {
	S(f) = s
	f()
}

main = () {
	_syn3_0 : S(Never) = S(() { () })
	f() <- (_syn3_0)
}
