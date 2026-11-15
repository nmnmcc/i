use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
a = (x : Bool) -> Nil { x |> (True) { use() } | (False) { b(True) } }
wrap = () { run(() { a(True) }) }
run = <N>(body : () <- N -> Nil) { body() }
S(N) = (() <- N -> Nil, (N) -> Nil)
b = (x : Bool) -> Nil { x |> (True) { use() } | (False) { a(True) } }
