wrap = () { run(() { a(True()) }) }
b = (x : Bool) -> () { x |> (True()) { use() } & (False()) { a(True()) } }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
a = (x : Bool) -> () { x |> (True()) { use() } & (False()) { b(True()) } }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (() <- N -> (), (N) -> ())
