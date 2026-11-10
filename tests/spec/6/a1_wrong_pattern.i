// old-syntax-check: E0108
// spec: 6.1
// expect: compile-error E0108
(main)
bad = (x) { x |> { -y -> 1 } }

@main
main = () {
  assert(bad(1) == 1)
  Nil
}
