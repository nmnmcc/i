S(A) = (value = A)
G(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : G(N)) {
	G(f, _) = s
	f()
}
run = (f) { f() }
with = <A, R, N>(x : A, f : () <- N -> R) {
	_syn3_0 : S(A) = S(value = x)
	f() <- (_syn3_0)
}
read = () <- (s : S(String)) { s.value <> "!" }

main = () {
	_syn3_1 : G(Never) = G(() { Nil }, (_ : Never) { Nil })
	{
		run(() { use() })
		with(1, read)
		Nil
	} <- (_syn3_1)
}
