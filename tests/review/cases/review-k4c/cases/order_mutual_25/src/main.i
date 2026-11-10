S(N) { S(() -> Nil ! N) }
a = (x: Bool) { x |> { True -> use() False -> b(True) } }
wrap = () { run(() { a(True) }) }
use(N) = (s ! S(N)) { S(f) = s f() }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
run(N) = (body: () -> Nil ! N) { body() }
