// spec: 2.2, 2.3, 2.4, 2.5
// expect: pass
(c = i/core)
(main)
(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })

main = () {
  assert(c.not(False))
  assert(even(10))
  assert(odd(9))
  Nil
}
