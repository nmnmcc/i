S(N) { S(() -> Nil ! N, (N) -> Nil) }
b = () { a() }
a(N) = () { ! S(N) = S(() { b() }, (_: N) { Nil }) use() }
use(N) = (s ! S(N)) { S(f, _) = s f() }
