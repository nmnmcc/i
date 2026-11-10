S(N) { S(() -> Nil ! N, (N) -> Nil) }
a(N) = () { ! S(N) = S(() { b() }, (_: N) { Nil }) use() }
b = () { a() }
use(N) = (s ! S(N)) { S(f, _) = s f() }
