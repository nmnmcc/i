S(A) { S(value: A) }
G(N) { G(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! G(N)) { G(f, _) = s f() }
run = (f) { f() }
with(A, R, N) = (x: A, f: () -> R ! N) { ! S(A) = S(value: x) f() }
read = (s ! S(String)) { s.value <> "!" }

main = () {
 ! G(Never) = G(() { Nil }, (_: Never) { Nil })
 run(() { use() })
 with(1, read)
 Nil
}
