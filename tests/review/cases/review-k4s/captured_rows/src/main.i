S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
capture = (body) {
	local = () { body() }
	local
}
example = () {
	_syn3_0 : S(Never) = S(() { () }, (_ : Never) { () })
	capture(() { use() })() <- (_syn3_0)
}
