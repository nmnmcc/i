// spec: 4.8, 7.4
// expect: compile-error E0501
strict = (s: String - "a") {
    s |> { "b" -> 1  _: String - "a" - "b" -> 2 }
}
f = (p: (String, Bool)) { p |> {
     ("a", True) -> 0
    (t, _) -> strict(t)
} }

main = () { f(("a", False)) }
