a = (x : Bool) { x |> (True) { use() } | (False) { b(True) } }
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
wrap = () { run(() { a(True) }) }
run = <N>(body : () <- N -> Nil) { body() }
S(N) = (() <- N -> Nil)
b = (x : Bool) { x |> (True) { use() } | (False) { a(True) } }
