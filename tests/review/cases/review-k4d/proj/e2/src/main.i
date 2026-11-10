State(A) { State(get: () -> A) }
read(A) = (s ! State(A)) { s.get() }
outer = (runner) {
  inner = () {
    c = () { read() }
    runner(c)
    c
  }
  inner
}
outer2 = (runner: (() -> Int ! State(Int)) -> Int ! State(Int)) {
  inner = () {
    c = () { read() }
    runner(c)
    c
  }
  inner
}
outer3 = (k) {
  inner = () { k(read()) }
  inner
}
outer4 = (k: (Int) -> Int) {
  inner = () { k(read()) }
  inner() + inner()
}
