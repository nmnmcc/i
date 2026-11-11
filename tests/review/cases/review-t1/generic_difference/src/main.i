drop(A) = (x: A | 0) -> A - 0 { x |> { 0 -> panic("zero")  rest -> rest } }
f = (x: Int) { drop(x) }
g = (x: "a" | "b") { drop(x) }

main = () { assert(f(3)==3) assert(g("b")=="b") Nil }
