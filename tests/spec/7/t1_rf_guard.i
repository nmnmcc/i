// spec: 4.8, 7.4
// expect: compile-error E0501
strict = (s: String - "a") {
    s |> { "b" -> 1  _: String - "a" - "b" -> 2 }
}
f = (s: String, c: Bool) { s |> { "a" ? c -> 0  t -> strict(t) } }

main = () { f("a", False) }
