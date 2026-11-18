S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
wrap = () { run(() { a(True) }) }
b = (x : Bool) -> () { x |> (True) { use() } & (False) { a(True) } }
a = (x : Bool) -> () { x |> (True) { use() } & (False) { b(True) } }
run = <N>(body : () <- N -> ()) { body() }
