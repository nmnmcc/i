S(N) = (() <- N -> Nil)
b = (x : Bool) { x |> (True) { use() } | (False) { a(True) } }
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(body : () <- N -> Nil) { body() }
a = (x : Bool) { x |> (True) { use() } | (False) { b(True) } }
wrap = () { run(() { a(True) }) }
