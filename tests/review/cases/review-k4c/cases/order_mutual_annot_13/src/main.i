wrap = () { run(() { a(True) }) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
S(N) { S(() -> Nil ! N, (N) -> Nil) }
b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }
a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }
