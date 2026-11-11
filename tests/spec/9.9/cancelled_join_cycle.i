// spec: 9.9
// expect: stop "panic: task: join of a cancelled task"
// runtime.md R4a: accepting cancellation publishes the outcome before cleanup;
// two shielded finals joining each other stop instead of creating a wait cycle.
(
	i/task
	i/resource
	i/fail
)


main = () {
	task.scope((s) {
		a: task.Channel(task.Task(Nil, String)) = task.channel()
		b: task.Channel(task.Task(Nil, String)) = task.channel()
		armed: task.Channel(Nil) = task.channel()
		left = task.go(s, () {
			other = task.receive(a)
			resource.ensure(() {
				task.send(armed, Nil)
				task.sync(task.never())
			}, () {
				fail.catch(() { task.join(other) })
				Nil
			})
		})
		right = task.go(s, () {
			other = task.receive(b)
			resource.ensure(() {
				task.send(armed, Nil)
				task.sync(task.never())
			}, () {
				fail.catch(() { task.join(other) })
				Nil
			})
		})
		task.send(a, right)
		task.send(b, left)
		task.receive(armed)
		task.receive(armed)
		fail.raise("outer")
	})
}
