A(N) { A(() -> Int ! N) }
B(N) { B(() -> Int ! N) }
ua(N) = (s ! A(N)) {
  A(f) = s
  f()
}
ub(N) = (s ! B(N)) {
  B(f) = s
  f()
}
bad = () {
  ! B(A(Never)) = B(() { ua() })
  ! A(B(A(Never))) = A(() { ub() + 1 })
  ua()
}
bad2(M) = () {
  ! B(M) = B(() { ua() })
  ! A(M) = A(() { ub() + 1 })
  ua()
}
grow = (x) { grow([x]) }
self = (x) { x(x) }
