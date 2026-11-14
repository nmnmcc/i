(
	map
	fold
)

map = <A, B>(xs : List(A), f : (A) -> B) -> List(B) {
	xs |> ([]) { [] } | ([x, ..rest]) { [f(x), ..map(rest, f)] }
}

fold = <A, B>(xs : List(A), init : B, f : (B, A) -> B) -> B {
	xs |> ([]) { init } | ([x, ..rest]) { fold(rest, f(init, x), f) }
}
