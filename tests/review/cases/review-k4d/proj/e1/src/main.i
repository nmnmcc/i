State(A) { State(get: () -> A) }
read(A) = (s ! State(A)) { s.get() }
outer = (runner) {
  inner = () {
    c = () { read() }
    runner(c)
    c
  }
  a = inner()
  b = inner()
  fa = () {
    ! State(Int) = State(() { 1 })
    a() + 1
  }
  fb = () {
    ! State(String) = State(() { "s" })
    b() <> "t"
  }
  (fa(), fb())
}

main = () {
  ! State(Int) = State(() { 5 })
  outer((c: () -> Int ! State(Int)) { c() })
  Nil
}
