(
	(Io) = i/io
)
Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int, m = (N) -> Nil)
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
	{
		h = () { run(fr) }
		k = () { run(h) + fw() }
		io.print(
			tell(k()),
		)
		io.print(
			tell(k() + fr()),
		) <- (S(f = () { 1 }, m = (_: Never) { Nil }))
	} <- (S(f = () { get() }, m = (_: Dep) { Nil }), Dep(v = 2))
}
