uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

run(A, N) = (body: () -> A ! N) { body() }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }


main = () {
  closed()
  Nil
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

S(N) { S(f: () -> Int ! N) }

pong = (n: Int) { ping(n) + uset(1) }

fr = () { run(() { use() }) }

loop = (x) { x(x) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

Dep { Dep(v: Int) }

fw = () { use() }

get = (d ! Dep) { d.v }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }
