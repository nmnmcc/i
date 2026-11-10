run(N) = (body: () -> Nil ! N) { body() }
b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }
use(N) = (s ! S(N)) { S(f, _) = s f() }
a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }
wrap = () { run(() { a(True) }) }
S(N) { S(() -> Nil ! N, (N) -> Nil) }
