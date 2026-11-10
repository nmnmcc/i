// §4.4: `{ A | B } & C` is `A & C | B & C`. A written intersection with a
// union in it is worked out, so the solver does not go round forever.
Box(a) { Box(item: a) }
unbox = (b: Box(a)) -> a { b.item }

call = (x: { Box(Int) | Nil } & Box(Int)) { unbox(x) }
bind = (x: { Box(Int) | Nil } & Box(Int)) {
  y: Box(_) = x
  y
}
field = (x: { Box(Int) | Nil } & Box(Int)) { x.item }
with_var = (x: a & { Box(Int) | Nil }) {
  case x {
    Box(item) -> item
    Nil -> 0
  }
}
empty = (x: { Box(Int) | Nil } & String) { x }
