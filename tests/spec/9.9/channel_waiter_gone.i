// spec: 9.9
// expect: stop "panic: task: channel is gone"
// runtime.md R3: an outer waiter still registered when an inner owner scope
// ends stops; it is not silently removed or treated as a closed channel.
// A scripted run fixes the waiter's turn before its owner's scope ends.
(
	i/task
	i/random
)

main = () {
	task.test(random.from_seed(1), [], () {
		task.scope((outer) {
			carrier: task.Channel(task.Channel(Int)) = task.channel()
			task.go(outer, () {
				c = task.receive(carrier)
				task.receive(c)
			})
			task.scope((_) {
				c: task.Channel(Int) = task.channel()
				task.send(carrier, c)
				task.yield()
			})
		})
	})
}
