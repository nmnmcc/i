// spec: 4.8
// expect: compile-error E0501

// F1: hidden-input matches cannot exclude a feasible branch.
(i/stdio, core = i/core)
(main)
State(A) = (get = () -> A)
cur = <S>(start: S) <- (state: State(S)) -> S { state.get() }
choose = (x: String) { "wrong" } | (x: { Int | String }) { 1 }
main = () {
  n: Int = choose(cur(0)) <- (State(get = () { "s" }))
  stdio.print(core.inspect(n + 1))
}
