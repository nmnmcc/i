(
	i/task
	i/resource
	i/fail
	i/request
	i/control
	i/core
	rng = i/random
	(Io) = i/io
)
spawn = (s, c, n: Int) {
	{ n == 0 } |>
		(True) { Nil }
		| (False) {
			task.go(s, () { task.send(c, n) })
			spawn(s, c, n - 1)
		}
}
// Every run is some serial order of task steps (spec §9.9); the senders may
// pair in any order, so check that each value arrives exactly once.
receive_all = (c, n: Int, seen: Int) {
	{ n == 0 } |>
		(True) { seen }
		| (False) {
			v = task.receive(c)
			bit = core.shift_left(1, v)
			assert(core.bit_and(seen, bit) == 0)
			receive_all(c, n - 1, core.bit_or(seen, bit))
		}
}

main = () <- (io: Io) {
	task.scope((s) {
		c: task.Channel(Int) = task.channel()
		spawn(s, c, 16)
		assert(receive_all(c, 16, 0) == core.shift_left(1, 16 + 1) - 2)
	})
	io.print("fifo-ok")
}
