// spec: 1.7
// expect: compile-error E0106
(main)
bad = (p) { p |> ((0, y)) { y } ((x, _)) { x } }

main = () {
  assert(bad((3, 4)) == 3)
  Nil
}
// old-syntax-check: E0106
