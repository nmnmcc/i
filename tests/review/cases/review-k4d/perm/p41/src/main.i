closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

run(A, N) = (body: () -> A ! N) { body() }

S(N) { S(f: () -> Int ! N) }

fr = () { run(() { use() }) }

fw = () { use() }


main = () {
  closed()
  Nil
}

get = (d ! Dep) { d.v }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

loop = (x) { x(x) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

pong = (n: Int) { ping(n) + uset(1) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

Dep { Dep(v: Int) }
