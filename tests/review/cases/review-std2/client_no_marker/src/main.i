(
	i/fail
)
S(E, N) = (() <- (Fail(E), ..N) -> Nil)
use = <E, N>() <- (s: S(E, N)) {
	S(f) = s
	f()
}
example = () { fail.catch(() { use() }) <- (S(() { fail.raise("x") })) }
