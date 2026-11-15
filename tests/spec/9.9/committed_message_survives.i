// spec: 9.9
// expect: pass
// runtime.md R3/6.4: a committed reply is no longer a channel registration;
// freeing the owner cannot erase the message already selected for its receiver.
(
	i/task
)

main = () {
	task.scope((outer) {
		carrier : task.Channel(task.Channel(Int)) = task.channel()
		reader = task.go(outer, () {
			c = task.receive(carrier)
			task.receive(c)
		})
		task.scope((_) {
			c : task.Channel(Int) = task.channel()
			task.send(carrier, c)
			task.send(c, 7)
		})
		assert(task.join(reader) == 7)
	})
}
