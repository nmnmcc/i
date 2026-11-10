get = (d ! Dep) { d.v }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

@main
main = () {
  closed()
  Nil
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

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

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

Dep { Dep(v: Int) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

loop = (x) { x(x) }

S(N) { S(f: () -> Int ! N) }

pong = (n: Int) { ping(n) + uset(1) }

fr = () { run(() { use() }) }

fw = () { use() }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

run(A, N) = (body: () -> A ! N) { body() }
