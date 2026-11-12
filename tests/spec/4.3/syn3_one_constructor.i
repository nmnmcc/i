// spec: 4.3
// expect: compile-error E0238
// migrate-syntax3: keep (a definition with two constructors is rejected)
(main)
Shape { Circle(Float) Square(Float) }

main = () {
  assert(True)
  Nil
}
