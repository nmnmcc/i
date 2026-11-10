// spec: 4.7
// expect: compile-error E0202
(main)
id = (x: Missing) { x }

@main
main = () {
  assert(True)
  Nil
}
