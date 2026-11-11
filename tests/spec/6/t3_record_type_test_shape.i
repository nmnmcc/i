// spec: 4.9, 7.1
// expect: compile-error E0501
(main)
f = (value: (name: Int) | (name: String)) -> Int {
  value |> {
    _: { (name: Int) } -> 1
    _ -> 2
  }
}

main = () { Nil }
