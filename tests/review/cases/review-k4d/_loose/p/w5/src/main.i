S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f = f, ..) = s
  f()
}
wrap = (body, svc) {
  ! S(_) = svc
  body()
}
