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
bad = () {
  ! B(A(Int, Never)) = B((d: Int) { ua(d - 1) + 1 })
  ! A(String, B(A(Int, Never))) = A((d: Int) { d |> {
    0 -> "str"
    _ -> {
      ub(d)
      "str"
    }
  } })
  ua(1)
}
@main
main = () {
  bad()
  Nil
}
