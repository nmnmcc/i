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
		gate: task.Channel(Nil) = task.channel()
		task.go(s, () {
			task.receive(gate)
			io.print("child")
		})
		io.print("body")
		task.send(gate, Nil)
	})
	io.print("after")
}
