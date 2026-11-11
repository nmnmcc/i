// spec: 4.9
// expect: pass
(main)
f(K) = (r: (K: Int), key: K) -> Int { 1 }
g(K) = (r: (K: Int), key: K) -> Int { f(r, key) }

main = () {
  assert(g((name: 9), "name") == 1)
  Nil
}
