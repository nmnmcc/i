get = (d ! Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }

fw = () { use() }

@main
main = () {
  closed()
  Nil
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

S(N) { S(f: () -> Int ! N) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

Dep { Dep(v: Int) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

loop = (x) { x(x) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

fr = () { run(() { use() }) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

run(A, N) = (body: () -> A ! N) { body() }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}
