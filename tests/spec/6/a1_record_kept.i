// old-syntax-check: E0114,E0101
// spec: 6.1
// expect: compile-error E0114
(main)
value = { name: 1 }

@main
main = () {
  assert(True)
  Nil
}
