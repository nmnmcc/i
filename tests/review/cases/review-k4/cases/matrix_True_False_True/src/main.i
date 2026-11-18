S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) -> () {
	S(f, _) = s
	f()
}
run = (body) { body() }
f = <N>() <- (s : S(N)) -> () { run(() { use() }) }

main = () {
	_syn3_0 : S(Never) = S(() { () }, (_ : Never) { () })
	f() <- (_syn3_0)
}
