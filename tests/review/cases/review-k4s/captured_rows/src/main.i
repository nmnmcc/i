S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
capture = (body) {
	local = () { body() }
	local
}
example = () { capture(() { use() })() <- (S(() { Nil }, (_: Never) { Nil })) }
