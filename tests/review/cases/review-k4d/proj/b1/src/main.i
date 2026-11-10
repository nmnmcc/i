S(A, N) { S(() -> A ! N) }
use(A, N) = (s ! S(A, N)) {
  S(f) = s
  f()
}
example = () {
  ! S(String, Never) = S(() { "a" })
  ! S(Int, S(String, Never)) = S(() {
    use()
    1
  })
  use()
}
@main
main = () {
  example()
  Nil
}
