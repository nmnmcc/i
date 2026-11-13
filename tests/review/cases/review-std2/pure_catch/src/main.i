(
	i/fail
)
S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
example = () { fail.catch(() { use() }) <- (S(() { Nil }, (_: Never) { Nil })) }
