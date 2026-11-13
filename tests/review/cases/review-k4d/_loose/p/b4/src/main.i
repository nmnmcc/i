Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }

main = () {
	run(() { use() })
	Nil
}
