b = (x : Bool) { x |> (True) { use() } & (False) { a(True) } }
S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
a = (x : Bool) { x |> (True) { use() } & (False) { b(True) } }
run = <N>(body : () <- N -> ()) { body() }
wrap = () { run(() { a(True) }) }
