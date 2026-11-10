// spec: 4.8, 7.1
// expect: compile-error E0501
Box(A) { Box(value: A) }
strict = (x: Box("a")) -> Int { Box(value: "a") = x  1 }
f = (b: Box(String)) { b |> { x: Box("a") -> strict(x)  _ -> 0 } }
@main
main = () { f(Box(value: "b")) }
