use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
S(N) = (() <- N -> Nil)
run = <N>(body : () <- N -> Nil) { body() }
wrap = () { run(() { a(True) }) }
a = (x : Bool) { x |> (True) { use() } | (False) { b(True) } }
b = (x : Bool) { x |> (True) { use() } | (False) { a(True) } }
