S(A) = (value = A)
G(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: G(N)) {
	G(f, _) = s
	f()
}
run = (f) { f() }
with = <A, R, N>(x: A, f: () <- N -> R) { f() <- (S(value = x)) }
read = () <- (s: S(String)) { s.value <> "!" }

main = () {
	{
		run(() { use() })
		with(1, read)
		Nil
	} <- (G(() { Nil }, (_: Never) { Nil }))
}
