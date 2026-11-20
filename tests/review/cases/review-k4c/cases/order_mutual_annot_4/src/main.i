b = (x : Bool) -> () { x |> (True()) { use() } & (False()) { a(True()) } }
a = (x : Bool) -> () { x |> (True()) { use() } & (False()) { b(True()) } }
wrap = () { run(() { a(True()) }) }
run = <N>(body : () <- N -> ()) { body() }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
S(N) = (() <- N -> (), (N) -> ())
