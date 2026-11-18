wrap = () { run(() { a(True) }) }
run = <N>(body : () <- N -> ()) { body() }
b = (x : Bool) -> () { x |> (True) { use() } & (False) { a(True) } }
S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
a = (x : Bool) -> () { x |> (True) { use() } & (False) { b(True) } }
