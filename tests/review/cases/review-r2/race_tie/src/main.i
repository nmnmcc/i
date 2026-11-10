(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)

@main
main = (io ! Io) {
v = task.race(() { resource.ensure(() { task.yield()
 1 }, () { io.print("left-final") }) }, () { resource.ensure(() { task.yield()
 2 }, () { task.yield()
 io.print("right-final") }) })
io.print(core.inspect(v))
}
