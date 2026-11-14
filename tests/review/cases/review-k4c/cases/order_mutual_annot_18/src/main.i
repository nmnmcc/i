a = (x : Bool) -> Nil { x |> (True) { use() } | (False) { b(True) } }
S(N) = (() <- N -> Nil, (N) -> Nil)
run = <N>(body : () <- N -> Nil) { body() }
b = (x : Bool) -> Nil { x |> (True) { use() } | (False) { a(True) } }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
wrap = () { run(() { a(True) }) }
