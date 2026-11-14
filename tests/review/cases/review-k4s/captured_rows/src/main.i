S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
capture = (body) {
	local = () { body() }
	local
}
example = () {
	_syn3_0 : S(Never) = S(() { Nil }, (_ : Never) { Nil })
	capture(() { use() })() <- (_syn3_0)
}
