use(N) = (s ! S(N)) { S(f) = s f() }
a = (x: Bool) { x |> { True -> use() False -> b(True) } }
S(N) { S(() -> Nil ! N) }
run(N) = (body: () -> Nil ! N) { body() }
wrap = () { run(() { a(True) }) }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
