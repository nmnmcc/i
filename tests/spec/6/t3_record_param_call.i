// spec: 4.9
// expect: pass
(main)
f(K) = (r: (K: Int), key: K) -> Int { 1 }
@main
main = () {
  assert(f((name: 9), "name") == 1)
  Nil
}
