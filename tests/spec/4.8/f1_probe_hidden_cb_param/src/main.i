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
