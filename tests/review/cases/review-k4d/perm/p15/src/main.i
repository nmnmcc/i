pong = (n: Int) { ping(n) + uset(1) }

run(A, N) = (body: () -> A ! N) { body() }

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

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

loop = (x) { x(x) }

get = (d ! Dep) { d.v }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

Dep { Dep(v: Int) }


main = () {
  closed()
  Nil
}

S(N) { S(f: () -> Int ! N) }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

fw = () { use() }

fr = () { run(() { use() }) }
