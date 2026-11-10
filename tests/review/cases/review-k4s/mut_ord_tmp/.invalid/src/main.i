// old-syntax-check: E0101,E0101
S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
D { D }
second(N) = (b: Bool, s ! S(N)) -> Nil ! N | S(N) { b |> { True -> h1 = () { h0 = () { first(False) } h0() } h1() False -> use() } }
first(N) = (b: Bool, s ! S(N)) -> Nil ! N | S(N) { b |> { True -> h1 = () { h0 = () { second(False) } h0() } h1() False -> use() } }
