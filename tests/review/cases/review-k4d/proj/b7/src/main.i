Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }

main = () {
	_syn3_0: S(Dep) = S(() { get() })
	{
		h = () { run(() { use() }) }
		k = () { run(h) }
		k()
		Nil
	} <- (_syn3_0)
}
