both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

S(N) { S(f: () -> Int ! N) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

Dep { Dep(v: Int) }

run(A, N) = (body: () -> A ! N) { body() }

fr = () { run(() { use() }) }

fw = () { use() }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

loop = (x) { x(x) }

get = (d ! Dep) { d.v }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}


main = () {
  closed()
  Nil
}
