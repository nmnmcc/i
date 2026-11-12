// old-syntax-check: E0101,E0001,E0101,E0001,E0101,E0001,E0101,E0101,E0001,E0001
S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A, N) = (body: () -> A ! N) { body() }
D { D }
test(N) = (s ! S(N)) { ! D = D (() { use() })() }
