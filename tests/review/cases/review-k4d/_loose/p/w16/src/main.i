S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  S(f, ..) = s
  f()
}
wrap = (svc) {
  ! S(_) = svc
  use()
}
