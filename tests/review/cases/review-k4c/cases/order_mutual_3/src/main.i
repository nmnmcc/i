run = <N>(body: () <- N -> Nil) { body() }
a = (x: Bool) { x |> (True) { use() } | (False) { b(True) } }
b = (x: Bool) { x |> (True) { use() } | (False) { a(True) } }
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
wrap = () { run(() { a(True) }) }
S(N) = (() <- N -> Nil)
