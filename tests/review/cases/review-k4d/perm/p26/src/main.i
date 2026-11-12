S(N) { S(f: () -> Int ! N) }

Dep { Dep(v: Int) }

run(A, N) = (body: () -> A ! N) { body() }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

get = (d ! Dep) { d.v }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

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


main = () {
  closed()
  Nil
}

fw = () { use() }

fr = () { run(() { use() }) }
