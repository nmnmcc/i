// spec: 6.12, 7.5
// expect: compile-error E0512
(main)
Shape { Circle(r: Int) Square(side: Int) }
bad = (s: Shape) { s |> { Circle(..) -> s.r } }


main = () {
  assert(True)
  Nil
}
