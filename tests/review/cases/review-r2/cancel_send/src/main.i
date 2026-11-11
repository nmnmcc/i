(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)


main = (io ! Io) {
r = fail.catch(() { task.scope((s) {
 c: task.Channel(Int) = task.channel()
 task.go(s, () { resource.ensure(() {
   resource.ensure(() { task.send(c, 1) }, () { io.print("inner")
 task.yield() })
 }, () { io.print("outer")
 task.yield()
 io.print("outer-done") }) })
 task.go(s, () { task.yield()
 fail.raise("boom") })
 task.sync(task.never())
}) })
io.print(core.inspect(r))
}
