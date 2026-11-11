// spec: 2.2
// expect: compile-error E0301
(main)
@m
(left, right) = (1, 2)


main = () {
  assert(left == 1)
  assert(right == 2)
  Nil
}
