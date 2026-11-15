// spec: 1.7
// expect: warning W0506
(main)
Pair = (Int, Int)
main = () {
  mk = Pair  (b, c) = Pair(1, 2)
  assert(mk == Pair(b, c))
}
