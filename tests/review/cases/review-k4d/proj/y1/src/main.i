Box(A) { Box(v: A) }
f(A) = (x: A) -> A { x |> {
  l: List(Int) -> x
  other -> other
} }
g(A) = (x: A) -> A { x |> {
  Box(v = v: 1) -> x
  other -> other
} }
h(A) = (x: A | Nil) -> A { x |> {
  Nil -> h(x)
  other -> other
} }
k(A) = (x: A | List(Int)) -> A { x |> {
  [] -> k(x)
  [n, ..] -> k([n + 1])
  other -> other
} }
a = () { f(["a"]) }
b = () { g(Box(v: "s")) }
c = () { k(["a"]) }
d = () { k(Box(v: 1)) }
