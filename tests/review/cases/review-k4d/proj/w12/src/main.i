S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  S(f, ..) = s
  f()
}
wrap(N) = (body: () -> Int ! N | S(N), svc: S(N)) {
  ! S(N) = svc
  body()
}
