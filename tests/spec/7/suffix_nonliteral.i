// old-syntax-check: E0111,E0101
// spec: 7.1
// expect: compile-error E0101
main = () { "axb" |> { r <> x -> r  _ -> "" } }
