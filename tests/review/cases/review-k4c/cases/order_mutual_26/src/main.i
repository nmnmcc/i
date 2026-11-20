a = (x : Bool) { x |> (True()) { use() } & (False()) { b(True()) } }
run = <N>(body : () <- N -> ()) { body() }
wrap = () { run(() { a(True()) }) }
S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
b = (x : Bool) { x |> (True()) { use() } & (False()) { a(True()) } }
