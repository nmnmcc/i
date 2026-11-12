// Generalized bindings are used from other modules with fresh types.
(
	map
	Pair(..)
	swap
)

Pair(A, B) = (left = A, right = B)

map = (xs, f) { xs |> ([]) { [] } | ([x, ..rest]) { [f(x), ..map(rest, f)] } }

swap = <A, B>(p: Pair(A, B)) { Pair(left = p.right, right = p.left) }
