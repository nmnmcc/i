a = (x: Bool) -> Nil { x |> (True) { use() } | (False) { b(True) } }
wrap = () { run(() { a(True) }) }
S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
b = (x: Bool) -> Nil { x |> (True) { use() } | (False) { a(True) } }
run = <N>(body: () <- N -> Nil) { body() }
