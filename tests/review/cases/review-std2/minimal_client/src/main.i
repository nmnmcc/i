(
	i/fail
)
S(E, N) = (() <- (Fail(E), ..N) -> (), (N) -> ())
use = <E, N>() <- (s : S(E, N)) {
	S(f, _) = s
	f()
}
example = () {
	fail.catch(() { use() }) <- (S(() { fail.raise("x") }, (_ : Never) { () }))
}
