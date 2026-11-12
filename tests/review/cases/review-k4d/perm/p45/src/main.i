both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

S(N) { S(f: () -> Int ! N) }

Dep { Dep(v: Int) }

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

fr = () { run(() { use() }) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

loop = (x) { x(x) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }


main = () {
  closed()
  Nil
}

pong = (n: Int) { ping(n) + uset(1) }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

fw = () { use() }

get = (d ! Dep) { d.v }
