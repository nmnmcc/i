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
	r = fail.catch(() {
		task.scope((s) {
			armed : task.Channel(()) = task.channel()
			joining : task.Channel(()) = task.channel()
			parent : task.Channel(()) = task.channel()
			t = task.go(s, () {
				resource.ensure(
					() {
						task.send(armed, ())
						task.sync(task.never())
					},
					() {
						task.receive(parent)
						io.print("child-final")
					},
				)
			})
			task.go(s, () {
				task.receive(armed)
				task.receive(joining)
				fail.raise("boom")
			})
			resource.ensure(
				() {
					task.send(joining, ())
					task.join(t)
				},
				() {
					io.print("parent-final")
					task.send(parent, ())
				},
			)
		})
	})
	io.print(core.inspect(r))
}
