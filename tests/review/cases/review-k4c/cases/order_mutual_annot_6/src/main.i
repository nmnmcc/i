S(N) = (() <- N -> Nil, (N) -> Nil)
b = (x: Bool) -> Nil { x |> (True) { use() } | (False) { a(True) } }
a = (x: Bool) -> Nil { x |> (True) { use() } | (False) { b(True) } }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <N>(body: () <- N -> Nil) { body() }
wrap = () { run(() { a(True) }) }
