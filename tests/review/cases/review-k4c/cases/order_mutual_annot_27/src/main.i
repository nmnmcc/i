wrap = () { run(() { a(True) }) }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
b = (x : Bool) -> () { x |> (True) { use() } & (False) { a(True) } }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (() <- N -> (), (N) -> ())
a = (x : Bool) -> () { x |> (True) { use() } & (False) { b(True) } }
