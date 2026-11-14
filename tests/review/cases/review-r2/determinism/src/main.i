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

// Fix the schedule while checking path-derived random values and output.
main = () <- (io : Io) {
	task.test(rng.from_seed(73), [], () {
		task.scope((s) {
			task.go(s, () {
				io.print(
					core.inspect(
						(task.path(), rng.word(task.random())),
					),
				)
				task.yield()
				io.print("a")
			})
			task.go(s, () {
				io.print(
					core.inspect(
						(task.path(), rng.word(task.random())),
					),
				)
				task.yield()
				io.print("b")
			})
			task.run(() {
				io.print(
					core.inspect(
						(task.path(), rng.word(task.random())),
					),
				)
			})
		})
	}) <- (task.top(rng.from_seed(73)))
}
