// §4.7: the bindings of a group are linked to their names, all of them,
// before any is generalized.
g = (y) { f(y) }
f = (x) {
  h(0)
  x
}
h = (z) { g(z) }
w: String = g(5)

f1 = (x) { g1(x) }
g1 = (y) {
  case y {
    0 -> 0
    _ -> f1(y - 1) + 1
  }
}
boom = () { f1(3) <> "x" }

// A tuple binding is a group too.
(f2, g2) = ((x) { g2(x) }, (y) { case y { 0 -> 0  _ -> f2(y - 1) + 1 } })
boom2 = () { f2("s") }
