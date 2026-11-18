wrap = () { run(() { a(True) }) }
b = (x : Bool) -> () { x |> (True) { use() } & (False) { a(True) } }
S(N) = (() <- N -> (), (N) -> ())
a = (x : Bool) -> () { x |> (True) { use() } & (False) { b(True) } }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
