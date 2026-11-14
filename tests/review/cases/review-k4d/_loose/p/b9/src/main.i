Dep = (v = Int)
get = () <- (d : Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <A, N>(body : () <- N -> A) { body() }

u1 = <N>() <- (s : S(N)) -> Int { use() }
u2 = () -> Int { use() }
u3 = () -> Int { use() + get() }

main = () {
	_syn3_0 : S(Dep) = S(() { get() })
	{
		u1() + u2() + u3()
		Nil
	} <- (_syn3_0)
}
