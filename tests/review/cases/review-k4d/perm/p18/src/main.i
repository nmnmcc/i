S(N) { S(f: () -> Int ! N) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}


main = () {
  closed()
  Nil
}

loop = (x) { x(x) }

run(A, N) = (body: () -> A ! N) { body() }

Dep { Dep(v: Int) }

pong = (n: Int) { ping(n) + uset(1) }

get = (d ! Dep) { d.v }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

fw = () { use() }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

fr = () { run(() { use() }) }
