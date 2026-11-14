// spec: 4.8
// expect: compile-error E0501

// F1: hidden-input matches cannot exclude a feasible branch.
(i/stdio, core = i/core, i/task)
(main)
choose = (x: String) { "wrong" } | (x: { Int | String }) { 1 }
main = () {
  r = task.ref(0)
  _ = task.update(r, (v) { ("s", 0) })
  v2 = task.update(r, (v) { (v, v) })
  n: Int = choose(v2)
  stdio.print(core.inspect(n + 1))
}
