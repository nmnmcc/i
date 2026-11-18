(
	i/fail
)
S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
example = () {
	fail.catch(() { use() }) <- (S(() { () }, (_ : Never) { () }))
}
