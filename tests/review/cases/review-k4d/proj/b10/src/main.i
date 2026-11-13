Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (() <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }

bad = <N>() <- (s: S(N)) -> Int {
	S(f) = s
	f()
}

main = () {
	{
		bad()
		Nil
	} <- (S(() { get() }))
}
