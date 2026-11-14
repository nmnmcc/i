Dep = (v = Int)
get = () <- (d : Dep) { d.v }
S(N) = (() <- N -> Int)
T(N) = (() <- N -> Int)
use = <N>() <- (s : S(N)) -> Int {
	S(f) = s
	f()
}
uset = <N>() <- (s : T(N)) {
	T(f) = s
	f()
}

main = () {
	_syn3_0 : S(Dep) = S(() { get() })
	_syn3_1 : T(S(Dep)) = T(() { use() })
	{
		uset()
		Nil
	} <- (_syn3_0, _syn3_1)
}
