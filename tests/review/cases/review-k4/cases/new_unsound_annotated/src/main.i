S(A, N) { S(() -> A ! N, (N) -> Nil) }
use(A, N) = (s ! S(A, N)) { S(f, _) = s f() }
run = (f) { f() }
read = () { run(() { use() }) <> "!" }
with(X, R, N) = (x: X, f: () -> R ! N | S(X, Never)) { ! S(X, Never) = S(() { x }, (_: Never) { Nil }) f() }

main = () { with(1, read) Nil }
