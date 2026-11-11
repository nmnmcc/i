S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
f = (n: Int) { n |> { 0 -> use() _ -> g(n - 1) } }
g = (n: Int) { n |> { 0 -> use() _ -> f(n - 1) } }

main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f(3) }
