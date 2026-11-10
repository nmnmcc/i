use(N) = (s ! S(N)) { S(f) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
a = (x: Bool) { x |> { True -> use() False -> b(True) } }
wrap = () { run(() { a(True) }) }
S(N) { S(() -> Nil ! N) }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
