a = (x : Bool) { x |> (True) { use() } & (False) { b(True) } }
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
b = (x : Bool) { x |> (True) { use() } & (False) { a(True) } }
S(N) = (() <- N -> ())
wrap = () { run(() { a(True) }) }
