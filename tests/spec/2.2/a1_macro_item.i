// spec: 2.2
// expect: compile-error E0234
(main)
make(1)

@main
main = () {
  assert(True)
  Nil
}
