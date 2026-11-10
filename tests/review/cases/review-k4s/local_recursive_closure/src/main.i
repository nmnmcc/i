S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
make(N) = () {
 () {
   ! S(N) = S(() { use() }, (_: N) { Nil })
   use()
 }
}
