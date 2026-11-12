(
	(Io) = i/io
)
Dep = (v = Int)
get = () <- (d: Dep) { d.v }
S(N) = (f = () <- N -> Int)
use = <N>() <- (s: S(N)) { s.f() }
run = <A, N>(body: () <- N -> A) { body() }
tell = (x: Int) {
	x |>
		(0) { "zero" }
		| (1) { "one" }
		| (2) { "two" }
		| (3) { "three" }
		| (4) { "four" }
		| (_) { "many" }
}

fw1 = <N>() <- (s: S(N)) -> Int { run(() { use() }) }
fw1i = () { run(() { use() }) }
fw2 = () -> Int { fw1() }
fw2i = () { fw1i() }
outer = <N>() -> Int {
	local: () <- (S(N), ..N) -> Int = () { use() }
	run(local)
}
outeri = () {
	local = () { use() }
	run(local)
}
dep3 = <N>() <- (s: S(N), d: Dep) -> Int { use() + d.v }
dep3i = () <- (d: Dep) { use() + d.v }
spec = () -> Int { use() }
half = () -> Int { use() }
wrap: <N>(() <- (S(N), ..N) -> Int) <- N -> Int = (body) {
	body() <- (S(f = () { 1 }))
}
wrapi = (body) { body() <- (S(f = () { 1 })) }
wrapj = (body) { body() <- (S(f = () { 1 })) }

main = () <- (io: Io) {
	a = wrap(() { use() })
	{
		b = wrap(() { use() + get() })
		c = wrapi(() { use() + get() })
		{
			d = fw2() +
				fw2i() -
				outer() -
				outeri() +
				dep3() -
				dep3i() +
				spec() -
				half()
			io.print(tell(a))
			io.print(tell(b))
			io.print(tell(c))
			io.print(tell(d))
		} <- (S(f = () { get() - 1 }))
	} <- (Dep(v = 3))
}
