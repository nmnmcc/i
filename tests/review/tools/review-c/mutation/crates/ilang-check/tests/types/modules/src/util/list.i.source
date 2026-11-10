// Generalized bindings are used from other modules with fresh types.
export (map, Pair(..), swap)

Pair(a, b) { Pair(left: a, right: b) }

map = (xs, f) {
  case xs {
    [] -> []
    [x, ..rest] -> [f(x), ..map(rest, f)]
  }
}

swap = (p: Pair(a, b)) { Pair(left: p.right, right: p.left) }
