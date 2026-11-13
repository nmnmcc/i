// spec: 7.4
// expect: compile-error E0516
f = (s: { "a" | "b" }) { s |> ("a" ? True) { 0 } | ("b") { 1 } }
