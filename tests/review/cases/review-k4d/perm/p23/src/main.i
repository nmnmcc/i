get = (d ! Dep) { d.v }

fw = () { use() }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

Dep { Dep(v: Int) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

@main
main = () {
  closed()
  Nil
}

S(N) { S(f: () -> Int ! N) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

run(A, N) = (body: () -> A ! N) { body() }

fr = () { run(() { use() }) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

pong = (n: Int) { ping(n) + uset(1) }

loop = (x) { x(x) }
