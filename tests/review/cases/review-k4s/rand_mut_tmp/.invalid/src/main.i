// old-syntax-check: E0101,E0001,E0101,E0001,E0101,E0001,E0101,E0001,E0101,E0001,E0101,E0101,E0001,E0001,E0001,E0101,E0001,E0001,E0101,E0001,E0001
S(N) { S(() -> Nil ! N, (N) -> Nil) }
T(N) { T(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
tuse(N) = (s ! T(N)) { T(f, _) = s f() }
run(A,N) = (body: () -> A ! N) { body() }
D { D }
f2(N) = (b: Bool, s ! S(N)) -> Nil ! N | S(N) | T(N) { b |> { True -> ! D = D run(() { h0 = () { f0(False) } h0() }) False -> { tuse() } } }
f1(N) = (b: Bool, t ! T(N)) -> Nil ! N | S(N) | T(N) { b |> { True -> run(() { h0 = () { f2(False) } h0() }) False -> { use() } } }
f0(N) = (b: Bool, s ! S(N), t ! T(N))  { b |> { True -> run(() { f1(False) }) False -> { use() tuse() } } }
