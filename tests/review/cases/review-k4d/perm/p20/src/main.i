use(N) = (s ! S(N)) {
  S(f) = s
  f()
}

T(A, N) { T(g: (A) -> A ! N, m: (N) -> Nil) }


main = () {
  closed()
  Nil
}

uset(A, N) = (x: A, t ! T(A, N)) -> A ! N | T(A, N) {
  T(g, ..) = t
  g(x)
}

Dep { Dep(v: Int) }

ping = (n: Int) { n |> {
  0 -> use()
  _ -> pong(n - 1)
} }

fr = () { run(() { use() }) }

S(N) { S(f: () -> Int ! N) }

loop = (x) { x(x) }

bad(N) = (s ! S(N)) -> Int ! N {
  S(f) = s
  f()
}

fw = () { use() }

run(A, N) = (body: () -> A ! N) { body() }

both(N) = () -> Int ! N | S(N) | T(Int, N) { pong(2) }

closed = () {
  ! Dep = Dep(1)
  ! T(Int, Dep) = T((x: Int) { x + get() }, (_: Dep) { Nil })
  ! S(Dep) = S(() { get() })
  both()
}

get = (d ! Dep) { d.v }

pong = (n: Int) { ping(n) + uset(1) }
