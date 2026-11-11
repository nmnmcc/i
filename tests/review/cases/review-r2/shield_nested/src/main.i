(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)


main = (io ! Io) {
r = fail.catch(() { task.scope((s) {
 gate: task.Channel(Nil) = task.channel()
 task.go(s, () { resource.ensure(() { task.send(gate, Nil)
 task.sync(task.never()) }, () {
  io.print("cleanup-start")
  task.run(() { task.scope((fresh) {
   task.go(fresh, () { resource.ensure(() { task.yield()
 io.print("fresh-child") }, () { task.yield()
 io.print("fresh-final") }) })
   task.yield()
  }) })
  io.print("cleanup-end")
 }) })
 task.receive(gate)
 fail.raise("outer")
}) })
io.print(core.inspect(r))
}
