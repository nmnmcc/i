b = (x: Bool) { x |> { True -> use() False -> a(True) } }
run(N) = (body: () -> Nil ! N) { body() }
use(N) = (s ! S(N)) { S(f) = s f() }
S(N) { S(() -> Nil ! N) }
wrap = () { run(() { a(True) }) }
a = (x: Bool) { x |> { True -> use() False -> b(True) } }
