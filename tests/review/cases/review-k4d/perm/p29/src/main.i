loop = (x) { x(x) }

S(N) { S(f: () -> Int ! N) }


main = () {
  closed()
  Nil
}

fr = () { run(() { use() }) }

fw = () { use() }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

Dep { Dep(v: Int) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

get = (d ! Dep) { d.v }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

run(A, N) = (body: () -> A ! N) { body() }

pong = (n: Int) { ping(n) + uset(1) }
