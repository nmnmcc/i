a(N) = () { ! S(N) = S(() { b() }, (_: N) { Nil }) use() }
b = () { a() }
S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
