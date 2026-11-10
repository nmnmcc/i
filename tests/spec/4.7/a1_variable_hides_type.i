// spec: 4.7
// expect: compile-error E0231
(main)
id(Int) = (x: Int) { x }

@main
main = () {
  assert(True)
  Nil
}
