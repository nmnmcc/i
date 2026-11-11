// spec: 4.9, 6.7
// expect: pass
(main)
L = (a: Int, b: String)
M = (a: String, b: Int)
U = (a: Int | String)
W = (b: Int | String)
dup(A) = (x: A, y: A) -> (A, A) { (x, y) }
use: (x: L, y: M) -> (U, W) = dup

main = () {
  (a, b) = use((a: 1, b: "s"), (a: "s", b: 1))
  assert(a.a == 1)
  assert(b.b == 1)
  Nil
}
