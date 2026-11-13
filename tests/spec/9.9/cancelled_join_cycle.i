// spec: 9.9
// expect: stop "panic: task: join of a cancelled task"
// runtime.md R4a: accepting cancellation publishes the outcome before cleanup;
// two shielded finals joining each other stop instead of creating a wait cycle.
(
	i/task
	i/resource
	i/fail
)


// §9.9 leaves open which of the two finals stops first, so both run the same
// code and stop at the same positions.
partner = (
	s: task.Scope(String)
	inbox: task.Channel(task.Task(Nil, String))
	armed: task.Channel(Nil)
) -> task.Task(Nil, String) ! task.Scheduler {
	task.go(s, () {
		other = task.receive(inbox)
		resource.ensure(() {
			task.send(armed, Nil)
			task.sync(task.never())
		}, () {
			fail.catch(() { task.join(other) })
			Nil
		})
	})
}

main = () {
	task.scope((s) {
		a: task.Channel(task.Task(Nil, String)) = task.channel()
		b: task.Channel(task.Task(Nil, String)) = task.channel()
		armed: task.Channel(Nil) = task.channel()
		left = partner(s, a, armed)
		right = partner(s, b, armed)
		task.send(a, right)
		task.send(b, left)
		task.receive(armed)
		task.receive(armed)
		fail.raise("outer")
	})
}
