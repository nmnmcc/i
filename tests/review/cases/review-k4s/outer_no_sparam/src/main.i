S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(body : () <- N -> A) { body() }
D = ()
test = () { run(() { use() }) <- (D) }
