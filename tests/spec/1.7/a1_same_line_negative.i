// old-syntax-check: E0101
// spec: 1.7
// expect: compile-error E0101
(main)
bad = (n) { n |> { 0 -> 10  -1 -> 20  _ -> 30 } }

main = () {
  assert(bad(-1) == 20)
  Nil
}
