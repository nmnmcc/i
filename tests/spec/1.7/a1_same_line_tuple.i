// spec: 1.7
// expect: compile-error E0106
(main)
bad = (p) { p |> { (0, y) -> y  (x, _) -> x } }

@main
main = () {
  assert(bad((3, 4)) == 3)
  Nil
}
