pong = (n: Int) { ping(n) + uset(1) }

run(A, N) = (body: () -> A ! N) { body() }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

S(N) { S(f: () -> Int ! N) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

fw = () { use() }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

Dep { Dep(v: Int) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

loop = (x) { x(x) }


main = () {
  closed()
  Nil
}

fr = () { run(() { use() }) }

get = (d ! Dep) { d.v }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }
