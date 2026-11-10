(
	(Io) = i/io
)
A(T, N) { A((Int) -> T ! N) }
B(N) { B((Int) -> Int ! N) }
ua(T, N) = (d: Int, s ! A(T, N)) {
  A(f) = s
  f(d)
}
ub(N) = (d: Int, s ! B(N)) {
  B(f) = s
  f(d)
}
tell = (x: Int) { x |> {
  0 -> "zero"
  1 -> "one"
  2 -> "two"
  _ -> "many"
} }
@main
main = (io ! Io) {
  ! B(A(Int, Never)) = B((d: Int) {
    io.print("in B")
    r = ua(d - 1) + 1
    io.print(tell(r))
    r
  })
  ! A(String, B(A(Int, Never))) = A((d: Int) {
    io.print("in A")
    d |> {
      0 -> "str"
      _ -> {
        ub(d)
        "str"
      }
    }
  })
  io.print(ua(1))
}
