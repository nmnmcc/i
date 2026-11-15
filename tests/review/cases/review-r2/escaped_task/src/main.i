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

main = () <- (io : Io) {
	t = task.scope((s) { task.go(s, () { 7 }) })
	io.print(
		core.inspect(task.join(t)),
	)
}
