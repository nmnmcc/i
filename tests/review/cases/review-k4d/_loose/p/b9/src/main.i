Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }

u1 = <N>() <- (s: S(N)) -> Int { use() }
u2 = () -> Int { use() }
u3 = () -> Int { use() + get() }

main = () {
	{
		u1() + u2() + u3()
		Nil
	} <- (S(() { get() }))
}
