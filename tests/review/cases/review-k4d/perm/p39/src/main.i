T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

loop = (x) { x(x) }

fw = () { use() }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

fr = () { run(() { use() }) }

pong = (n: Int) { ping(n) + uset(1) }

Dep { Dep(v: Int) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

run(A, N) = (body: () -> A ! N) { body() }


main = () {
  closed()
  Nil
}

get = (d ! Dep) { d.v }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

S(N) { S(f: () -> Int ! N) }
