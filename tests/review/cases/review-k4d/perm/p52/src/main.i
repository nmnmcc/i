closed = () {
  ! Dep = Dep(v: 1)
  ! T(Int, Dep) = T(g: (x: Int) { x + get() }, m: (_: Dep) { Nil })
  ! S(Dep) = S(f: () { get() })
  both()
}

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g = g, ..) = t
  g(x)
}


main = () {
  closed()
  Nil
}

fr = () { run(() { use() }) }

Dep { Dep(v: Int) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f = f) = s
  f()
}

use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}

S(N) { S(f: () -> Int ! N) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

fw = () { use() }

run(A, N) = (body: () -> A ! N) { body() }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

loop = (x) { x(x) }

get = (d ! Dep) { d.v }
