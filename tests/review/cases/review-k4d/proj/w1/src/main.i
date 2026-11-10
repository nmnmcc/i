S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f, ..) = s
  f()
}
wrap(N) = (body: () -> Int ! N | S(N)) -> Int ! N {
  ! S(N) = S(() { 1 })
  body()
}
