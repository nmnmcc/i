S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f, ..) = s
  f()
}
wrap(N) = (svc: S(N)) -> Int ! N {
  ! S(N) = svc
  use()
}
