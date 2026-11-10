loop = (x) { x(x) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

fw = () { use() }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

pong = (n: Int) { ping(n) + uset(1) }

@main
main = () {
  closed()
  Nil
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

run(A, N) = (body: () -> A ! N) { body() }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

S(N) { S(f: () -> Int ! N) }

fr = () { run(() { use() }) }

Dep { Dep(v: Int) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

get = (d ! Dep) { d.v }
