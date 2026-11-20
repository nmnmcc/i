a = (x : Bool) -> () { x |> (True()) { use() } & (False()) { b(True()) } }
wrap = () { run(() { a(True()) }) }
S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
b = (x : Bool) -> () { x |> (True()) { use() } & (False()) { a(True()) } }
run = <N>(body : () <- N -> ()) { body() }
