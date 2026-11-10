a = (x: Bool) { x |> { True -> use() False -> b(True) } }
run(N) = (body: () -> Nil ! N) { body() }
S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
wrap = () { run(() { a(True) }) }
