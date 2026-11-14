S(N) = (() <- N -> Nil)
wrap = () { run(() { a(True) }) }
b = (x : Bool) { x |> (True) { use() } | (False) { a(True) } }
run = <N>(body : () <- N -> Nil) { body() }
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
a = (x : Bool) { x |> (True) { use() } | (False) { b(True) } }
