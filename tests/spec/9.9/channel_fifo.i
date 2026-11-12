// spec: 9.9
// expect: pass
// runtime.md 6.4: FIFO pending partners; new ready alternatives use source order;
// committing one alternative removes all its losing registrations.
(
	i/task
)

main = () {
	task.scope((s) {
		c: task.Channel(Int) = task.channel()
		task.go(s, () { task.send(c, 1) })
		task.go(s, () { task.send(c, 2) })
		receiver = task.go(s, () { (task.receive(c), task.receive(c)) })
		assert(task.join(receiver) == (1, 2))
	})
	task.scope((s) {
		a: task.Channel(Int) = task.channel()
		b: task.Channel(Int) = task.channel()
		task.go(s, () { task.send(a, 11) })
		task.go(s, () { task.send(b, 22) })
		reader = task.go(s, () {
			first = task.sync(
				task.choose([task.receive_event(b), task.receive_event(a)]),
			)
			(first, task.receive(a))
		})
		assert(task.join(reader) == (22, 11))
	})
}
