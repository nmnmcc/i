S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run = (f) { f() }
id = (x) { x }
f(N) = ()  { f = () { use() } f() }

main = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) f() }
