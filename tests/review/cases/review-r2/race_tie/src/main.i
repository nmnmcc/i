(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)

// Fix the schedule for this particular race and its cleanup trace.

main = (io ! Io) {
v = task.test(rng.from_seed(1), [], () { task.race(() { resource.ensure(() { task.yield()
 1 }, () { io.print("left-final") }) }, () { resource.ensure(() { task.yield()
 2 }, () { task.yield()
 io.print("right-final") }) }) })
io.print(core.inspect(v))
}
