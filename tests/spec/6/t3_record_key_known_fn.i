// spec: 4.8, 4.9
// expect: pass
(main)
Box(A) { Box(item: A) }
apply(K) = (b: Box((K: Int)), key: K, f: (Box((K: Int))) -> Int) -> Int { f(b) }
@main
main = () {
  assert(apply(Box(item: (a: 1)), "a", (x) { x.item.a }) == 1)
  Nil
}
