ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

fr = () { run(() { use() }) }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
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

run(A, N) = (body: () -> A ! N) { body() }


main = () {
  closed()
  Nil
}

Dep { Dep(v: Int) }

pong = (n: Int) { ping(n) + uset(1) }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

loop = (x) { x(x) }

get = (d ! Dep) { d.v }

S(N) { S(f: () -> Int ! N) }

fw = () { use() }
