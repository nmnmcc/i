Dep = (v = Int)
get = () <- (d : Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}

main = () {
	_syn3_0 : S(Never) = S(() { get() })
	{
		use()
		Nil
	} <- (_syn3_0)
}
