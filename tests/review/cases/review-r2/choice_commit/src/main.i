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

main = () <- (io: Io) {
	task.scope((s) {
		c: task.Channel(Int) = task.channel()
		task.go(s, () { task.send(c, 7) })
		task.yield()
		x = task.sync(
			task.choose(
				[
					task.map(task.receive_event(c), (x) {
						task.yield()
						io.print("mapper")
						x + 1
					}),
					task.map(task.receive_event(c), (_) {
						panic("losing mapper")
					}),
					task.always(99),
				],
			),
		)
		assert(x == 8)
		assert(
			task.sync(
				task.choose([task.receive_event(c), task.always(100)]),
			) ==
				100,
		)
		io.print("once")
	})
}
