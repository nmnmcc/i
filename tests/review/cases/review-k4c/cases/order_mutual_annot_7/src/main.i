S(N) = (() <- N -> (), (N) -> ())
a = (x : Bool) -> () { x |> (True()) { use() } & (False()) { b(True()) } }
run = <N>(body : () <- N -> ()) { body() }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
wrap = () { run(() { a(True()) }) }
b = (x : Bool) -> () { x |> (True()) { use() } & (False()) { a(True()) } }
