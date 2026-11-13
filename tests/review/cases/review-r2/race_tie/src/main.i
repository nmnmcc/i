(
	i/task
	i/resource
	(Io) = i/io
)

main = () <- (io : Io) {
	task.scope((s) {
		armed : task.Channel(Nil) = task.channel()
		release : task.Channel(Nil) = task.channel()
		finals : task.Channel(Int) = task.channel()
		acknowledged : task.Channel(Nil) = task.channel()
		collected = task.go(s, () {
			a = task.receive(finals)
			b = task.receive(finals)
			pair = (a, b)
			assert(pair == (1, 2) || pair == (2, 1))
			io.print("left-final")
			io.print("right-final")
			task.send(acknowledged, Nil)
			task.send(acknowledged, Nil)
		})
		raced = task.go(s, () {
			task.race(
				() {
					resource.ensure(
						() {
							task.send(armed, Nil)
							task.receive(release)
							1
						},
						() {
							task.send(finals, 1)
							task.receive(acknowledged)
						},
					)
				},
				() {
					resource.ensure(
						() {
							task.send(armed, Nil)
							task.receive(release)
							2
						},
						() {
							task.yield()
							task.send(finals, 2)
							task.receive(acknowledged)
						},
					)
				},
			)
		})
		// Install both finals before allowing either body to finish. The spec's
		// workers paragraph permits either winner and either order of final steps.
		task.receive(armed)
		task.receive(armed)
		task.send(release, Nil)
		// Neither final can finish until the collector has heard from both.
		task.send(release, Nil)
		winner = task.join(raced)
		assert(winner == 1 || winner == 2)
		io.print("winner")
		task.join(collected)
	})
}
