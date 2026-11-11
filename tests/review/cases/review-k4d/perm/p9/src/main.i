Dep { Dep(v: Int) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

loop = (x) { x(x) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

pong = (n: Int) { ping(n) + uset(1) }

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

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

fr = () { run(() { use() }) }

run(A, N) = (body: () -> A ! N) { body() }

fw = () { use() }

S(N) { S(f: () -> Int ! N) }

get = (d ! Dep) { d.v }


main = () {
  closed()
  Nil
}
