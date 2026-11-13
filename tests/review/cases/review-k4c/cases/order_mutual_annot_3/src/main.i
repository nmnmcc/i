run = <N>(body: () <- N -> Nil) { body() }
b = (x: Bool) -> Nil { x |> (True) { use() } | (False) { a(True) } }
wrap = () { run(() { a(True) }) }
S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
a = (x: Bool) -> Nil { x |> (True) { use() } | (False) { b(True) } }
