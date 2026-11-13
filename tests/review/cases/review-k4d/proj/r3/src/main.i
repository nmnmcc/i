(
	(Io) = i/io
)
Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) { s.f() }
run = <A, N>(body: () <- N -> A) { body() }
fw = () { use() }
fr = () { run(() { use() }) }
tell = (x: Int) {
	x |>
		(0) { "zero" }
		| (1) { "one" }
		| (2) { "two" }
		| (3) { "three" }
		| (4) { "four" }
		| (_) { "many" }
}

main = () {
	_syn3_0: S(Dep) = S(f = () { get() })
	{
		h = () { run(fr) }
		k = () { run(h) + fw() }
		k()
		Nil
	} <- (_syn3_0)
}
