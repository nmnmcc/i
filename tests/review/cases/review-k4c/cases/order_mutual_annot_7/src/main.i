S(N) { S(() -> Nil ! N, (N) -> Nil) }
a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }
run(N) = (body: () -> Nil ! N) { body() }
use(N) = (s ! S(N)) { S(f, _) = s f() }
wrap = () { run(() { a(True) }) }
b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }
