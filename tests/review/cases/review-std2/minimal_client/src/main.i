(i/fail)
S(E,N) { S(() -> Nil ? E ! N, (N) -> Nil) }
use(E,N) = (s ! S(E,N)) { S(f, _) = s f() }
example = () {
 ! S(String,Never) = S(() { fail.raise("x") }, (_: Never) { Nil })
 fail.catch(() { use() })
}
