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
		task.go(s, () {
			task.yield()
			io.print("child")
		})
		io.print("body")
	})
	io.print("after")
}
