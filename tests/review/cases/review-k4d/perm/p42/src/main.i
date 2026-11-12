
main = () {
  closed()
  Nil
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

S(N) { S(f: () -> Int ! N) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

run(A, N) = (body: () -> A ! N) { body() }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

fw = () { use() }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

fr = () { run(() { use() }) }

loop = (x) { x(x) }

Dep { Dep(v: Int) }

pong = (n: Int) { ping(n) + uset(1) }

get = (d ! Dep) { d.v }
