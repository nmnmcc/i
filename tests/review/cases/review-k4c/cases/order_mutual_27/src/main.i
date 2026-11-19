b = (x : Bool) { x |> (True) { use() } & (False) { a(True) } }
S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
wrap = () { run(() { a(True) }) }
run = <N>(body : () <- N -> ()) { body() }
a = (x : Bool) { x |> (True) { use() } & (False) { b(True) } }
