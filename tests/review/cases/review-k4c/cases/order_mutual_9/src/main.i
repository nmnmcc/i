a = (x: Bool) { x |> { True -> use() False -> b(True) } }
S(N) { S(() -> Nil ! N) }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
run(N) = (body: () -> Nil ! N) { body() }
use(N) = (s ! S(N)) { S(f) = s f() }
wrap = () { run(() { a(True) }) }
