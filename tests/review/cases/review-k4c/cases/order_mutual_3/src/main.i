run(N) = (body: () -> Nil ! N) { body() }
a = (x: Bool) { x |> { True -> use() False -> b(True) } }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
use(N) = (s ! S(N)) { S(f) = s f() }
wrap = () { run(() { a(True) }) }
S(N) { S(() -> Nil ! N) }
