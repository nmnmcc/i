run(A, N) = (body: () -> A ! N) { body() }

loop = (x) { x(x) }

fw = () { use() }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

pong = (n: Int) { ping(n) + uset(1) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

get = (d ! Dep) { d.v }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }


main = () {
  closed()
  Nil
}

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

Dep { Dep(v: Int) }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

fr = () { run(() { use() }) }

S(N) { S(f: () -> Int ! N) }
