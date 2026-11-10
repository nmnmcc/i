b = (x: Bool) { x |> { True -> use() False -> a(True) } }
S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
wrap = () { run(() { a(True) }) }
run(N) = (body: () -> Nil ! N) { body() }
a = (x: Bool) { x |> { True -> use() False -> b(True) } }
