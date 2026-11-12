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
spin = (n: Int) -> Never { spin(n + 1) }

main = () <- (io: Io) {
	task.scope((s) {
		task.go(s, () { io.print("child") })
		spin(0)
	})
}
