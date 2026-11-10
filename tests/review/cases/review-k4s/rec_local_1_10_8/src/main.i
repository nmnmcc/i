S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
bad(N) = (s ! S(N)) -> Nil ! N { f: () -> Nil ! N | S(N) = () { use() } f() }
