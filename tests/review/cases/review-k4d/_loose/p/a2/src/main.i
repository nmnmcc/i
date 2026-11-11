S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(N) = (body: () -> Nil ! N) { body() }
example = () {
  ! S(Never) = S(() { Nil }, (_: Never) { Nil })
  run(() { use() })
}

main = () { example() }
