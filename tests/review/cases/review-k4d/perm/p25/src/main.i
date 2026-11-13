both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

S(N) { S(f: () -> Int ! N) }

loop = (x) { x(x) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

run(A, N) = (body: () -> A ! N) { body() }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

pong = (n: Int) { ping(n) + uset(1) }


main = () {
  closed()
  Nil
}

fr = () { run(() { use() }) }

get = (d ! Dep) { d.v }

Dep { Dep(v: Int) }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

fw = () { use() }
