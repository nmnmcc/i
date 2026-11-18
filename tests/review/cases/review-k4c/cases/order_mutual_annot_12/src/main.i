a = (x : Bool) -> () { x |> (True) { use() } & (False) { b(True) } }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
S(N) = (() <- N -> (), (N) -> ())
run = <N>(body : () <- N -> ()) { body() }
wrap = () { run(() { a(True) }) }
b = (x : Bool) -> () { x |> (True) { use() } & (False) { a(True) } }
