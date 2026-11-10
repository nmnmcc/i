S(N) { S(() -> Nil ! N) }
use(N) = (s: S(N)) { S(f) = s f() }
@main
example = () { use(S(() { Nil })) }
