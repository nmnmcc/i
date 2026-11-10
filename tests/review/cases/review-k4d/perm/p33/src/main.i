run(A, N) = (body: () -> A ! N) { body() }

fw = () { use() }

pong = (n: Int) { ping(n) + uset(1) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

loop = (x) { x(x) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

get = (d ! Dep) { d.v }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

@main
main = () {
  closed()
  Nil
}

Dep { Dep(v: Int) }

S(N) { S(f: () -> Int ! N) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

fr = () { run(() { use() }) }
