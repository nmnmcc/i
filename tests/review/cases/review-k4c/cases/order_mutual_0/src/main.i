S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
a = (x: Bool) { x |> { True -> use() False -> b(True) } }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
run(N) = (body: () -> Nil ! N) { body() }
wrap = () { run(() { a(True) }) }
