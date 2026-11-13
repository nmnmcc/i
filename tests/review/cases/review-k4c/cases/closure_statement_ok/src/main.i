Dep = (value = Int)
get = () <- (d: Dep) { d.value }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <N>(f: () <- N -> Int) { f() }

main = () {
	{
		local = () {
			_syn3_0: S(Dep) = S(() { get() })
			run(() { use() }) <- (_syn3_0)
		}
		local()
		Nil
	} <- (Dep(value = 42))
}
