// spec: 4.9, 7.1
// expect: pass
(main)
f = (value: Record | Int) -> Int {
  value |> {
    _: Record -> 1
    _: Int -> 2
  }
}

main = () {
  assert(f((x: 5)) == 1)
  assert(f(8) == 2)
  Nil
}
