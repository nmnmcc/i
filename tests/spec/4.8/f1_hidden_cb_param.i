// spec: 4.8
// expect: compile-error E0501

// F1: hidden-input matches cannot exclude a feasible branch.
(i/stdio, core = i/core)
(main)
repeat = <S, N>(start: S, step: (S) <- N -> S) -> S { step(step(start)) }
choose = (x: String) { "wrong" } | (x: { Int | String }) { 1 }
main = () {
  _ = repeat(0, (x) {
    n: Int = choose(x)
    stdio.print(core.inspect(n + 1))
    "s"
  })
  Nil
}
