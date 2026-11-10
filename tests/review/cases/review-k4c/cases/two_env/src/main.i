S(A, B) { S(() -> Nil ! A, () -> Nil ! B) }
use(A, B) = (s ! S(A, B)) { S(f, g) = s f() g() }
wrap(A,B) = () { local: () -> Nil ! A | B | S(A, B) = () { use() } local() }
