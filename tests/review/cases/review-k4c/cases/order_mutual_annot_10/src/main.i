S(N) = (() <- N -> (), (N) -> ())
run = <N>(body : () <- N -> ()) { body() }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
a = (x : Bool) -> () { x |> (True()) { use() } & (False()) { b(True()) } }
wrap = () { run(() { a(True()) }) }
b = (x : Bool) -> () { x |> (True()) { use() } & (False()) { a(True()) } }
