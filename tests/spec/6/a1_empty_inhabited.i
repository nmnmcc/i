// spec: 4.8, 6.12
// expect: compile-error E0501
(main)

@main
main = () {
  1 |> {}
  Nil
}
