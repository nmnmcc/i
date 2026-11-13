Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }

main = () {
	g = () {
		_syn3_0: S(Never) = S(() { 3 })
		use() <- (_syn3_0)
	}
	g()
	Nil
}
