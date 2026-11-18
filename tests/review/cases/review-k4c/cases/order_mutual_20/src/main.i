a = (x : Bool) { x |> (True) { use() } & (False) { b(True) } }
b = (x : Bool) { x |> (True) { use() } & (False) { a(True) } }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
wrap = () { run(() { a(True) }) }
