a = (x: Bool) { x |> (True) { use() } | (False) { b(True) } }
wrap = () { run(() { a(True) }) }
S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <N>(body: () <- N -> Nil) { body() }
b = (x: Bool) { x |> (True) { use() } | (False) { a(True) } }
