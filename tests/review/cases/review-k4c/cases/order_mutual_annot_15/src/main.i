run = <N>(body: () <- N -> Nil) { body() }
S(N) = (() <- N -> Nil, (N) -> Nil)
wrap = () { run(() { a(True) }) }
a = (x: Bool) -> Nil { x |> (True) { use() } | (False) { b(True) } }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
b = (x: Bool) -> Nil { x |> (True) { use() } | (False) { a(True) } }
