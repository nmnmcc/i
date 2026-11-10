a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }
use(N) = (s ! S(N)) { S(f, _) = s f() }
S(N) { S(() -> Nil ! N, (N) -> Nil) }
run(N) = (body: () -> Nil ! N) { body() }
wrap = () { run(() { a(True) }) }
b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }
