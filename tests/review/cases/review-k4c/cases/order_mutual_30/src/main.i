a = (x: Bool) { x |> { True -> use() False -> b(True) } }
use(N) = (s ! S(N)) { S(f) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
S(N) { S(() -> Nil ! N) }
wrap = () { run(() { a(True) }) }
