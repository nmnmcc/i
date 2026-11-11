fr = () { run(() { use() }) }

fw = () { use() }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

pong = (n: Int) { ping(n) + uset(1) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

loop = (x) { x(x) }


main = () {
  closed()
  Nil
}

get = (d ! Dep) { d.v }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

S(N) { S(f: () -> Int ! N) }

Dep { Dep(v: Int) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

run(A, N) = (body: () -> A ! N) { body() }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
