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
bad1 = () {
  ! A(B(Never)) = A(() { ub() + 1 })
  1
}
mk = () -> A(B(A(Never))) { A(() { ub() + 1 }) }
bad2 = () {
  ! B(A(Never)) = B(() { ua() })
  ! A(B(A(Never))) = A(() { ub() + 1 })
  2
}
bad3 = () {
  ! B(A(Never)) = B(() { ua() })
  ! A(B(A(Never))) = A(() { ub() + 1 })
  ua()
}
