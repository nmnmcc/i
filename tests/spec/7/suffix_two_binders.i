// old-syntax-check: E0108
// spec: 7.1
// expect: compile-error E0108
main = () { "axb" |> { a <> "x" <> b -> a <> b  _ -> "" } }
