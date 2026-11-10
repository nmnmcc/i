wrap = () { run(() { a(True) }) }
b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }
S(N) { S(() -> Nil ! N, (N) -> Nil) }
a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
