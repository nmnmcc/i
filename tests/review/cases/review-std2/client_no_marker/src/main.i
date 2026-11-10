(i/fail)
S(E,N) { S(() -> Nil ? E ! N) }
use(E,N) = (s ! S(E,N)) { S(f) = s f() }
example = () {
 ! S(String,Never) = S(() { fail.raise("x") })
 fail.catch(() { use() })
}
