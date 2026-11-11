// Design counterexample; not run. Runtime gives ([1], [2], [3]).
(i/task)

main = () {
  task.scope((s) {
    a = task.go(s, task.path)
    nested = task.run(task.path)
    b = task.go(s, task.path)
    (task.join(a), nested, task.join(b))
  })
}
