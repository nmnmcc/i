(
	(Io) = i/io
)
Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) {
	S(f = f) = s
	f()
}
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
		) <- (S(f = () { 1 }))
	} <- (S(f = () { get() }), Dep(v = 2))
}
