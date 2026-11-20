a = (x : Bool) { x |> (True()) { use() } & (False()) { b(True()) } }
wrap = () { run(() { a(True()) }) }
S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
b = (x : Bool) { x |> (True()) { use() } & (False()) { a(True()) } }
