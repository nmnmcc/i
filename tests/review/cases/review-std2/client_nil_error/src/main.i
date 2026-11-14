(
	i/fail
)
S(E, N) = (() <- (Fail(E), ..N) -> Nil, (N) -> Nil)
use = <E, N>() <- (s : S(E, N)) {
	S(f, _) = s
	f()
}
example = () {
	fail.catch(() { use() }) <- (S(() { Nil }, (_ : Never) { Nil }))
}
