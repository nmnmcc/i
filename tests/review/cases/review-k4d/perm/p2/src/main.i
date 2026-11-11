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

S(N) { S(f: () -> Int ! N) }

pong = (n: Int) { ping(n) + uset(1) }

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

fw = () { use() }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

Dep { Dep(v: Int) }

loop = (x) { x(x) }

fr = () { run(() { use() }) }


main = () {
  closed()
  Nil
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

get = (d ! Dep) { d.v }

run(A, N) = (body: () -> A ! N) { body() }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}
