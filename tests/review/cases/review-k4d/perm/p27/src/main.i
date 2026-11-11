use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

loop = (x) { x(x) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

fr = () { run(() { use() }) }

Dep { Dep(v: Int) }

S(N) { S(f: () -> Int ! N) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }


main = () {
  closed()
  Nil
}

run(A, N) = (body: () -> A ! N) { body() }

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }

pong = (n: Int) { ping(n) + uset(1) }

get = (d ! Dep) { d.v }

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

fw = () { use() }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}
