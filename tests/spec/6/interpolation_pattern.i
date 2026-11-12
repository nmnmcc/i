// old-syntax-check: E0101
// spec: 7.1
// expect: compile-error E0101
main = () { "a" |> { "\("a")" -> Nil } }
