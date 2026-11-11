State(A) { State(get: () -> A) }
read(A) = (s ! State(A)) { s.get() }
outer = () {
  inner = () { read() }
  fa = () {
    ! State(Int) = State(() { 1 })
    inner() + 1
  }
  fb = () {
    ! State(String) = State(() { "x" })
    inner() <> "y"
  }
  (fa(), fb())
}

main = () {
  outer()
  Nil
}
