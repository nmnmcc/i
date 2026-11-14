// spec: 4.8
// expect: compile-error E0501

// F1: hidden-input matches cannot exclude a feasible branch.
(i/stdio, core = i/core)
(main)
make = <S>(start: S) -> ((S) -> S) -> S { (step) { step(start) } }
choose = (x: String) { "wrong" } | (x: { Int | String }) { 1 }
main = () {
  m = make(0)
  n: Int = choose(m((x) { "s" }))
  stdio.print(core.inspect(n + 1))
}
