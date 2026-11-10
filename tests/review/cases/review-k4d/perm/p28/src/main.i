Dep { Dep(v: Int) }

loop = (x) { x(x) }

fr = () { run(() { use() }) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

pong = (n: Int) { ping(n) + uset(1) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

run(A, N) = (body: () -> A ! N) { body() }

get = (d ! Dep) { d.v }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

fw = () { use() }

S(N) { S(f: () -> Int ! N) }

@main
main = () {
  closed()
  Nil
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
