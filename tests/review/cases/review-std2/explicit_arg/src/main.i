S(N) { S(() -> Nil ! N) }
use(N) = (s: S(N)) { S(f) = s f() }

main = () { use(S(() { Nil })) }
