run = <N>(body : () <- N -> ()) { body() }
S(N) = (() <- N -> (), (N) -> ())
a = (x : Bool) -> () { x |> (True) { use() } & (False) { b(True) } }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
wrap = () { run(() { a(True) }) }
b = (x : Bool) -> () { x |> (True) { use() } & (False) { a(True) } }
