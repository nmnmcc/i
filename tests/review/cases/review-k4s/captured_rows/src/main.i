S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
capture = (body) { local = () { body() } local }
example = () {
 ! S(Never) = S(() { Nil }, (_: Never) { Nil })
 capture(() { use() })()
}
