both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

fr = () { run(() { use() }) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }


main = () {
  closed()
  Nil
}

pong = (n: Int) { ping(n) + uset(1) }

fw = () { use() }

S(N) { S(f: () -> Int ! N) }

Dep { Dep(v: Int) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

loop = (x) { x(x) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

get = (d ! Dep) { d.v }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

run(A, N) = (body: () -> A ! N) { body() }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}
