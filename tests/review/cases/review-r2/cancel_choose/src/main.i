(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)

@main
main = (io ! Io) {
task.scope((outer) {
 a: task.Channel(Int) = task.channel()
 b: task.Channel(Int) = task.channel()
 c: task.Channel(Int) = task.channel()
 r = fail.catch(() { task.scope((s) {
   task.go(s, () { resource.ensure(() { task.sync(task.choose([
     task.send_event(a, 1), task.send_event(b, 2), task.send_event(c, 3)
   ])) }, () { task.yield()
 io.print("withdrawn") }) })
   task.go(s, () { task.yield()
 fail.raise("boom") })
   task.sync(task.never())
 }) })
 assert(r == Error("boom"))
 assert(task.sync(task.choose([task.receive_event(a), task.receive_event(b), task.receive_event(c), task.always(99)])) == 99)
 io.print("clean")
})
}
