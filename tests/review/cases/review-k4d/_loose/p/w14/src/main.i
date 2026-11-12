S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  S(f = f, ..) = s
  f()
}
wrap(N) = (svc: S(N)) -> Int ! N {
  ! S(N) = svc
  use()
}
