(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)
deep = (n: Int) -> Nil ! task.Scheduler { { n == 0 } |> { True -> task.yield()
 False -> task.scope((s) { task.go(s, () { Nil })
 deep(n - 1) }) } }

@main
main = (io ! Io) {
deep(50)
io.print("deep-ok")
}
