// spec: 4.8, 7.4
// expect: compile-error E0516
f = (n: Int) { n |> { 0 -> 2  1 -> 2 } }
