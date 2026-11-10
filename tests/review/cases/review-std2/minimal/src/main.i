S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
example = () { ! S(Never) = S(() { Nil }) use() }
