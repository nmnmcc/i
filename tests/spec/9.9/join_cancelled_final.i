// spec: 9.9
// expect: stop "panic: task: join of a cancelled task"
// runtime.md review test S / R4a: a shielded final receives a dropped child;
// joining it stops immediately instead of waiting for a fabricated result.
(
	i/task
	i/resource
	i/fail
)


main = () {
	task.scope((s) {
		gate: task.Channel(Nil) = task.channel()
		task.go(s, () {
			resource.ensure(() {
				task.send(gate, Nil)
				task.sync(task.never())
			}, () {
				t = task.go(s, () { Nil })
				fail.catch(() { task.join(t) })
				Nil
			})
		})
		task.receive(gate)
		fail.raise("boom")
	})
}
