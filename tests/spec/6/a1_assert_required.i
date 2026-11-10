// spec: 4.8, 6.12
// expect: compile-error E0501
(main)
pure: (Int) -> Int = { 0 -> 7 }

@main
main = () {
  assert(True)
  Nil
}
