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
(Io) = i/io
@main
main = (io ! Io) {
  ! B(Never) = B(() { 1 })
  ! A(B(Never)) = A(() { ub() + 1 })
  io.print(int_text(ua()))
}
int_text = (n: Int) { n |> {
  2 -> "two"
  _ -> "other"
} }
