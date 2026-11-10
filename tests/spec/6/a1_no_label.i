// spec: 6.12
// expect: compile-error E0507
(main)
classify = { 0 -> 0  n -> n }

@main
main = () {
  assert(classify(x: 1) == 1)
  Nil
}
