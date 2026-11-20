S(N) = (() <- N -> ())
a = (x : Bool) { x |> (True()) { use() } & (False()) { b(True()) } }
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
wrap = () { run(() { a(True()) }) }
b = (x : Bool) { x |> (True()) { use() } & (False()) { a(True()) } }
run = <N>(body : () <- N -> ()) { body() }
