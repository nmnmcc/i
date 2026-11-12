S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f = f, ..) = s
  f()
}
wrap(N) = (svc: S(N)) {
  ! S(N) = svc
  use()
}
