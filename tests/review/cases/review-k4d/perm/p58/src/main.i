both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

S(N) { S(f: () -> Int ! N) }

loop = (x) { x(x) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

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

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

pong = (n: Int) { ping(n) + uset(1) }

fw = () { use() }

fr = () { run(() { use() }) }

run(A, N) = (body: () -> A ! N) { body() }

get = (d ! Dep) { d.v }

Dep { Dep(v: Int) }


main = () {
  closed()
  Nil
}
