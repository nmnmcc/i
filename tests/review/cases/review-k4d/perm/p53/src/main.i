get = (d ! Dep) { d.v }

fr = () { run(() { use() }) }

Dep { Dep(v: Int) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

fw = () { use() }

S(N) { S(f: () -> Int ! N) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

run(A, N) = (body: () -> A ! N) { body() }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

@main
main = () {
  closed()
  Nil
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

loop = (x) { x(x) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

pong = (n: Int) { ping(n) + uset(1) }
