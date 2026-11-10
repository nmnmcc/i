// spec: 10.1, 10.3
// expect: compile-error E0318
@external("test:math/v1", "add")
add: (a: Int, b: Int) -> 1 | 2
@main
main = () { add(2, 3) |> { 1 -> Nil  2 -> Nil } }
