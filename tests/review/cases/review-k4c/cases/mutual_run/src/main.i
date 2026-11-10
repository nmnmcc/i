S(N) { S(() -> Int ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f,_) = s f() }
a(N) = (x: Int) -> Int ! N | S(N) { x |> { 0 -> use() _ -> b(x-1) } }
b(N) = (x: Int) -> Int ! N | S(N) { x |> { 0 -> use() _ -> a(x-1) } }
@main
main = () { ! S(Never) = S(() { 42 }, (_: Never) { Nil }) assert(a(4) == 42) assert(b(3) == 42) Nil }
