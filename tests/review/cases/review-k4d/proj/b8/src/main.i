Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(() -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
run(A, N) = (body: () -> A ! N) { body() }

ping(N) = (n: Int, s ! S(N)) -> Int ! N | S(N) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }
pong = (n: Int) { ping(n) }
ping2 = (n: Int) { n |> {
  0 -> use()
  _ -> pong2(n - 1)
} }
pong2 = (n: Int) { ping2(n) }
@main
main = () {
  ! S(Dep) = S(() { get() })
  pong2(3)
  pong(3)
  Nil
}
