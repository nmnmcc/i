(i/task, i/resource, i/fail, i/request, i/control, i/core, rng = i/random, (Io) = i/io)
spawn = (s, c, n: Int) { { n == 0 } |> { True -> Nil
 False -> { task.go(s, () { task.send(c, n) })
 spawn(s, c, n - 1) } } }
receive_all = (c, n: Int) { { n == 0 } |> { True -> Nil
 False -> { assert(task.receive(c) == n)
 receive_all(c, n - 1) } } }


main = (io ! Io) {
task.scope((s) { c: task.Channel(Int) = task.channel()
 spawn(s, c, 128)
 receive_all(c, 128) })
io.print("fifo-ok")
}
