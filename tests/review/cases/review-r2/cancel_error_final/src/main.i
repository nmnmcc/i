(
	i/task
	i/resource
	i/fail
	i/core
	(Io) = i/io
)

main = () <- (io: Io) {
	r = fail.catch(() {
		task.scope((s) {
			gate: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(
					() { fail.raise("child") },
					() {
						io.print("final-start")
						task.send(gate, Nil)
						task.yield()
						io.print("final-end")
					},
				)
			})
			task.receive(gate)
			fail.raise("parent")
		})
	})
	io.print(core.inspect(r))
}
