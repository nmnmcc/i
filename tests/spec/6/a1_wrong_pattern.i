// spec: 6.1
// expect: compile-error E0108
(main)
bad = (x) { x |> (-y) { 1 } | (_) { 0 } }

main = () {
  assert(bad(1) == 1)
  Nil
}
// old-syntax-check: E0108
