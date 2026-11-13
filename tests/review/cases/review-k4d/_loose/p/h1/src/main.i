Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
c1 = () -> Int { use() }
c2: () <- (S(Dep)) -> Int = () { use() }
c3 = () {
	cb: () <- (S(Dep)) -> Int = () { use() }
	cb
}
c4 = () -> Int { get() }
c5 = () <- (s: S(Dep)) -> Int {
	S(f) = s
	f()
}
c6 = (h: () <- (Dep, S(Dep)) -> Int) -> () <- (S(Dep)) -> Int { h }
c7 = (h: () <- (Dep, S(Dep)) -> Int, k: (() <- (S(Dep)) -> Int) -> Int) { k(h) }
