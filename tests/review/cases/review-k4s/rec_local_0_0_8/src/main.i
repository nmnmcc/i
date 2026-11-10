S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
bad(N) = ()  { f: () -> Nil ! N | S(N) = () { use() } f() }
