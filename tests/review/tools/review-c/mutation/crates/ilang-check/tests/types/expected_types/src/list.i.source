export (map, fold)

map = (xs: List(a), f: (a) -> b) -> List(b) {
  case xs {
    [] -> []
    [x, ..rest] -> [f(x), ..map(rest, f)]
  }
}

fold = (xs: List(a), init: b, f: (b, a) -> b) -> b {
  case xs {
    [] -> init
    [x, ..rest] -> fold(rest, f(init, x), f)
  }
}
