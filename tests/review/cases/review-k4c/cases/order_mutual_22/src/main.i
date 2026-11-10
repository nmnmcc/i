use(N) = (s ! S(N)) { S(f) = s f() }
wrap = () { run(() { a(True) }) }
run(N) = (body: () -> Nil ! N) { body() }
S(N) { S(() -> Nil ! N) }
a = (x: Bool) { x |> { True -> use() False -> b(True) } }
b = (x: Bool) { x |> { True -> use() False -> a(True) } }
