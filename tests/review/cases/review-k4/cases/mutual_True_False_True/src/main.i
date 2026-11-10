S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(body, _) = s body() }
g(M) = (n: Int, s ! S(M))  { n |> { 0 -> use() _ -> f(n - 1) } }
f(N) = (n: Int, s ! S(N)) -> Nil ! N | S(N) { n |> { 0 -> use() _ -> g(n - 1) } }
@main
main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f(3) }
