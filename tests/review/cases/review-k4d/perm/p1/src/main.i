T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

loop = (x) { x(x) }

fw = () { use() }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

fr = () { run(() { use() }) }

get = (d ! Dep) { d.v }

Dep { Dep(v: Int) }


main = () {
  closed()
  Nil
}

run(A, N) = (body: () -> A ! N) { body() }

S(N) { S(f: () -> Int ! N) }

pong = (n: Int) { ping(n) + uset(1) }
