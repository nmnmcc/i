// spec: 7.1
// expect: compile-error E0501
main = () { 1 |> { r <> "x" -> r } }
