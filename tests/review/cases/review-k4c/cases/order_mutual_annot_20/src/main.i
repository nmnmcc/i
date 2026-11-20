use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
a = (x : Bool) -> () { x |> (True()) { use() } & (False()) { b(True()) } }
wrap = () { run(() { a(True()) }) }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (() <- N -> (), (N) -> ())
b = (x : Bool) -> () { x |> (True()) { use() } & (False()) { a(True()) } }
