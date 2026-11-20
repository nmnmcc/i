S(N) = (() <- N -> ())
b = (x : Bool) { x |> (True()) { use() } & (False()) { a(True()) } }
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
a = (x : Bool) { x |> (True()) { use() } & (False()) { b(True()) } }
wrap = () { run(() { a(True()) }) }
