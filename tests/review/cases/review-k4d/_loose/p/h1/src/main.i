Dep = (v = Int)
get = () <- (d : Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
c1 : () <- (Fail(Never), S(Dep)) -> Int = () <- (fail : Fail(Never)) -> Int {
	use()
}
c2 : () <- (S(Dep)) -> Int = () { use() }
c3 = () {
	cb : () <- (S(Dep)) -> Int = () { use() }
	cb
}
c4 : () <- (Fail(Never), S(Dep)) -> Int = () <- (fail : Fail(Never)) -> Int {
	get()
}
c5 : () <- (Fail(Never), S(Dep)) -> Int = () <- (
	s : S(Dep),
	fail : Fail(Never),
) -> Int {
	S(f) = s
	f()
}
c6 = (h : () <- (Dep, S(Dep)) -> Int) -> () <- (S(Dep)) -> Int { h }
c7 = (h : () <- (Dep, S(Dep)) -> Int, k : (() <- (S(Dep)) -> Int) -> Int) {
	k(h)
}
