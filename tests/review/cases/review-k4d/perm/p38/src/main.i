fr = () { run(() { use() }) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}


main = () {
  closed()
  Nil
}

get = (d ! Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

run(A, N) = (body: () -> A ! N) { body() }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

loop = (x) { x(x) }

S(N) { S(f: () -> Int ! N) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

Dep { Dep(v: Int) }

fw = () { use() }
