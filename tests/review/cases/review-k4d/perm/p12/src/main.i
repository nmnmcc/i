uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

S(N) { S(f: () -> Int ! N) }


main = () {
  closed()
  Nil
}

fr = () { run(() { use() }) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

loop = (x) { x(x) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

Dep { Dep(v: Int) }

run(A, N) = (body: () -> A ! N) { body() }

pong = (n: Int) { ping(n) + uset(1) }

fw = () { use() }

get = (d ! Dep) { d.v }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}
