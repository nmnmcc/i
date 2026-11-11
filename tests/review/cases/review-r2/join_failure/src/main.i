(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)

@main
main = (io ! Io) {
r = fail.catch(() { task.scope((s) {
 t = task.go(s, () { resource.ensure(() { task.sync(task.never()) }, () { task.yield()
 io.print("child-final") }) })
 task.go(s, () { task.yield()
 fail.raise("boom") })
 resource.ensure(() { task.join(t) }, () { io.print("parent-final") })
}) })
io.print(core.inspect(r))
}
