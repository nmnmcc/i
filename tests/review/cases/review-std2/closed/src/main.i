S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
closed: () -> Nil ! S(Never) = use

main = () { ! S(Never) = S(() { Nil }) closed() }
