(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)


main = (io ! Io) {
r = fail.catch(() { task.scope((s) {
 armed: task.Channel(Nil) = task.channel()
 joining: task.Channel(Nil) = task.channel()
 parent: task.Channel(Nil) = task.channel()
 t = task.go(s, () { resource.ensure(() { task.send(armed, Nil)
 task.sync(task.never()) }, () { task.receive(parent)
 io.print("child-final") }) })
 task.go(s, () { task.receive(armed)
 task.receive(joining)
 fail.raise("boom") })
 resource.ensure(() { task.send(joining, Nil)
 task.join(t) }, () { io.print("parent-final")
 task.send(parent, Nil) })
}) })
io.print(core.inspect(r))
}
