// spec: 4.7, 9.8
// expect: compile-error E0501
// multicore.md D13: a fresh key is not generalized between unrelated types.
(i/key)


main = () {
  k = key.new()
  x = key.wrap(k, 1)
  key.unwrap(k, x) |> {
    key.Found(text) -> assert(text <> "wrong" == "")
    key.Missing -> Nil
  }
}
