b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }
a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }
wrap = () { run(() { a(True) }) }
run(N) = (body: () -> Nil ! N) { body() }
use(N) = (s ! S(N)) { S(f, _) = s f() }
S(N) { S(() -> Nil ! N, (N) -> Nil) }
