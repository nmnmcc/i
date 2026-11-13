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
	control.boundary((b) {
		task.scope((s) {
			t = task.go(s, () {
				assert(
					core.not(core.portable(b)),
				)
				7
			})
			assert(task.join(t) == 7)
		})
	})
	io.print("pinned")
}
