Dep = (value = Int)
get = () <- (d: Dep) { d.value }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <N>(f: () <- N -> Int) { f() }

main = () {
	local = () { run(() { use() }) <- (S(() { get() })) }
	local()
}
