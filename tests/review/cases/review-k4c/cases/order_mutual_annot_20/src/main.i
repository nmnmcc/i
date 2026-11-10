use(N) = (s ! S(N)) { S(f, _) = s f() }
a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }
wrap = () { run(() { a(True) }) }
run(N) = (body: () -> Nil ! N) { body() }
S(N) { S(() -> Nil ! N, (N) -> Nil) }
b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }
