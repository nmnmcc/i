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

main = () <- (io: Io) {
	_syn3_0: S(Dep) = S(f = () { get() })
	{
		h = () { run(fr) }
		k = () { run(h) + fw() }
		io.print(
			tell(k()),
		)
		_syn3_1: S(Never) = S(f = () { 1 })
		io.print(
			tell(k() + fr()),
		) <- (_syn3_1)
	} <- (_syn3_0, Dep(v = 2))
}
