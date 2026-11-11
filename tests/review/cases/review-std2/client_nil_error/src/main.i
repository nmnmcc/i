(i/fail)
S(E,N) { S(() -> Nil ^ E ! N, (N) -> Nil) }
use(E,N) = (s ! S(E,N)) { S(f, _) = s f() }
example = () {
 ! S(Nil,Never) = S(() { Nil }, (_: Never) { Nil })
 fail.catch(() { use() })
}
