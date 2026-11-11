(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)


main = (io ! Io) {
control.boundary((b) { task.scope((s) { task.go(s, () { b })
 Nil }) })
}
