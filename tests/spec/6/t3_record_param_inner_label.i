// spec: 4.9
// expect: pass
(main)
f(K) = (r: Record, key: K) -> Int {
  h = (s: (K: Int)) { s }
  0
}
@main
main = () { assert(f((), "name") == 0)  Nil }
