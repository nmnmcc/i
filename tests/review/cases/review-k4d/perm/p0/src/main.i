Dep { Dep(v: Int) }

get = (d ! Dep) { d.v }

S(N) { S(f: () -> Int ! N) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

run(A, N) = (body: () -> A ! N) { body() }

fw = () { use() }

fr = () { run(() { use() }) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

pong = (n: Int) { ping(n) + uset(1) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

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

loop = (x) { x(x) }


main = () {
  closed()
  Nil
}
