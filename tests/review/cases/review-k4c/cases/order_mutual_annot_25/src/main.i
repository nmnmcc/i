S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <N>(body : () <- N -> Nil) { body() }
wrap = () { run(() { a(True) }) }
b = (x : Bool) -> Nil { x |> (True) { use() } | (False) { a(True) } }
a = (x : Bool) -> Nil { x |> (True) { use() } | (False) { b(True) } }
