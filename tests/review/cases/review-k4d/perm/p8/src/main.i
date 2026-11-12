fr = () { run(() { use() }) }

closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

run(A, N) = (body: () -> A ! N) { body() }


main = () {
  closed()
  Nil
}

pong = (n: Int) { ping(n) + uset(1) }

loop = (x) { x(x) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

get = (d ! Dep) { d.v }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

S(N) { S(f: () -> Int ! N) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}

fw = () { use() }

Dep { Dep(v: Int) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }
