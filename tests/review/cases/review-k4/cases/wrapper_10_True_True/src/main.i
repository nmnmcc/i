S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = (f) { f() }
id = (x) { x }
f = <N>() <- (s: S(N)) -> Nil { True |> (True) { use() } | (False) { use() } }

main = () {
	_syn3_0: S(Never) = S(() { Nil }, (_: Never) { Nil })
	f() <- (_syn3_0)
}
