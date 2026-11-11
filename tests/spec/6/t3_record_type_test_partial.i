// spec: 4.9, 7.1
// expect: compile-error E0512
(main)
f = (value: Record | Int) -> Int {
  value |> {
    _: { (name: Int) } -> 1
    _: Int -> 2
  }
}
@main
main = () { Nil }
