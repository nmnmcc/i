wrap = () { run(() { a(True()) }) }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
S(N) = (() <- N -> (), (N) -> ())
b = (x : Bool) -> () { x |> (True()) { use() } & (False()) { a(True()) } }
a = (x : Bool) -> () { x |> (True()) { use() } & (False()) { b(True()) } }
