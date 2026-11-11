fw = () { use() }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

get = (d ! Dep) { d.v }

loop = (x) { x(x) }


main = () {
  closed()
  Nil
}

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

Dep { Dep(v: Int) }

S(N) { S(f: () -> Int ! N) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

fr = () { run(() { use() }) }

run(A, N) = (body: () -> A ! N) { body() }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
