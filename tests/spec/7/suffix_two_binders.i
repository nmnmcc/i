// old-syntax-check: E0111,E0101
// spec: 7.1
// expect: compile-error E0108
main = () { "axb" |> { a <> "x" <> b -> a <> b  _ -> "" } }
