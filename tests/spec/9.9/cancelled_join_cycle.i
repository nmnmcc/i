// spec: 9.9
// expect: stop "panic: task: join of a cancelled task"
// runtime.md R4a: accepting cancellation publishes the outcome before cleanup;
// two shielded finals joining each other stop instead of creating a wait cycle.
(
	i/task
	i/resource
	i/fail
)

// Which of the two tasks stops first is not fixed when the program runs on
// several workers (spec §9.9: timing may choose which failures precede
// cancellation), so both run the same code: the stop names the same place.
waiter = (inbox, armed) {
	() {
		other = task.receive(inbox)
		resource.ensure(
			() {
				task.send(armed, Nil)
				task.sync(task.never())
			},
			() {
				fail.catch(() { task.join(other) })
				Nil
			},
		)
	}
}

main = () {
	task.scope((s) {
		a : task.Channel(task.Task(Nil, String)) = task.channel()
		b : task.Channel(task.Task(Nil, String)) = task.channel()
		armed : task.Channel(Nil) = task.channel()
		left = task.go(s, waiter(a, armed))
		right = task.go(s, waiter(b, armed))
		task.send(a, right)
		task.send(b, left)
		task.receive(armed)
		task.receive(armed)
		fail.raise("outer")
	})
}
