// runtime §6.2: race cancels and cleans up the loser before returning.
(
	i/task
	i/resource
	i/core
	(Io) = i/io
)

forever = () -> Never {
	task.yield()
	forever()
}

main = () <- (io : Io) {
	answer = task.race(
		() {
			task.yield()
			7
		},
		() { resource.ensure(forever, () { io.print("loser cancelled") }) },
	)
	io.print(core.inspect(answer))
}
