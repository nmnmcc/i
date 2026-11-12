both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }


main = () {
  closed()
  Nil
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

get = (d ! Dep) { d.v }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

run(A, N) = (body: () -> A ! N) { body() }

Dep { Dep(v: Int) }

fr = () { run(() { use() }) }

S(N) { S(f: () -> Int ! N) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

pong = (n: Int) { ping(n) + uset(1) }

fw = () { use() }

loop = (x) { x(x) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}
