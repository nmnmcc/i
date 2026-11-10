run(A, N) = (body: () -> A ! N) { body() }

fw = () { use() }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

get = (d ! Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }

loop = (x) { x(x) }

Dep { Dep(v: Int) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

S(N) { S(f: () -> Int ! N) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

@main
main = () {
  closed()
  Nil
}

fr = () { run(() { use() }) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
