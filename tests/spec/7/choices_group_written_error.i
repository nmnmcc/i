// spec: 4.8, 6.12, 7.3
// expect: compile-error E0501
s = "b"
f = () -> Int { s |> { "a" | "c" -> 1 } }
