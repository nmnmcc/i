run(A, N) = (body: () -> A ! N) { body() }

get = (d ! Dep) { d.v }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

fr = () { run(() { use() }) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

loop = (x) { x(x) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

pong = (n: Int) { ping(n) + uset(1) }

S(N) { S(f: () -> Int ! N) }


main = () {
  closed()
  Nil
}

Dep { Dep(v: Int) }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

fw = () { use() }
