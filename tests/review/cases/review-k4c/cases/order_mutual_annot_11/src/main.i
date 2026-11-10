run(N) = (body: () -> Nil ! N) { body() }
use(N) = (s ! S(N)) { S(f, _) = s f() }
a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }
S(N) { S(() -> Nil ! N, (N) -> Nil) }
b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }
wrap = () { run(() { a(True) }) }
