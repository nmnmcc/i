(
	(Io) = i/io
)
Dep = (v = Int)
get = () <- (d : Dep) { d.v }
S(N) = (f = () <- N -> Int)
use = <N>() <- (s : S(N)) { s.f() }
run = <A, N>(body : () <- N -> A) { body() }
fw = () { use() }
fr = () { run(() { use() }) }
tell = (x : Int) {
	x |>
		(0) { "zero" }
		| (1) { "one" }
		| (2) { "two" }
		| (3) { "three" }
		| (4) { "four" }
		| (_) { "many" }
}

T(N) = (f = () <- N -> Int, m = (N) -> Nil)
uset = <N>() <- (s : T(N)) { s.f() }

main = () {
	_syn3_0 : S(Never) = S(f = () { 1 })
	_syn3_1 : T(S(Never)) = T(f = () { fw() }, m = (_ : S(Never)) { Nil })
	_syn3_2 : S(Dep) = S(f = () { get() })
	{
		uset()
		Nil
	} <- (_syn3_0, _syn3_1, _syn3_2)
}
