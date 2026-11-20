a = (x : Bool) { x |> (True()) { use() } & (False()) { b(True()) } }
run = <N>(body : () <- N -> ()) { body() }
S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
b = (x : Bool) { x |> (True()) { use() } & (False()) { a(True()) } }
wrap = () { run(() { a(True()) }) }
