// typed macro: 15.7.4
(m = spec/macros)
f = (x: (a: Int) | (b: String)) { m.m(x) }
main = () {
  assert(f((a: 42)) == (a: 42))
  assert(f((b: "x")) == (b: "x"))
}
