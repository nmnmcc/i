@main
main = () {
  closed()
  Nil
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

fr = () { run(() { use() }) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

get = (d ! Dep) { d.v }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

run(A, N) = (body: () -> A ! N) { body() }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

pong = (n: Int) { ping(n) + uset(1) }

fw = () { use() }

loop = (x) { x(x) }

S(N) { S(f: () -> Int ! N) }

Dep { Dep(v: Int) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}
