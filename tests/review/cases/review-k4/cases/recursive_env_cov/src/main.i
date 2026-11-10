S(N) { S(() -> Nil ! N) }
bad(N) = (s ! S(N)) -> Nil ! N { S(f) = s f() }
