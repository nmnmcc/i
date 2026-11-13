// spec: 7.1
// expect: compile-error E0101
main = () { "axb" |> (a <> "x" <> b) { a <> b } | (_) { "" } }
// old-syntax-check: E0101
