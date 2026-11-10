loop = (x) { x(x) }

pong = (n: Int) { ping(n) + uset(1) }

run(A, N) = (body: () -> A ! N) { body() }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

@main
main = () {
  closed()
  Nil
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

Dep { Dep(v: Int) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

get = (d ! Dep) { d.v }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

fw = () { use() }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

fr = () { run(() { use() }) }

S(N) { S(f: () -> Int ! N) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}
