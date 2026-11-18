a = (x : Bool) { x |> (True) { use() } & (False) { b(True) } }
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
S(N) = (() <- N -> ())
b = (x : Bool) { x |> (True) { use() } & (False) { a(True) } }
wrap = () { run(() { a(True) }) }
run = <N>(body : () <- N -> ()) { body() }
