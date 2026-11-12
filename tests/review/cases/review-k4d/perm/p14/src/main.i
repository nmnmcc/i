bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

run(A, N) = (body: () -> A ! N) { body() }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

pong = (n: Int) { ping(n) + uset(1) }

Dep { Dep(v: Int) }

S(N) { S(f: () -> Int ! N) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

fr = () { run(() { use() }) }

loop = (x) { x(x) }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}


main = () {
  closed()
  Nil
}

get = (d ! Dep) { d.v }

fw = () { use() }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }
