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
		gate : task.Channel(()) = task.channel()
		task.go(s, () {
			resource.ensure(
				() {
					task.send(gate, ())
					task.sync(task.never())
				},
				() {
					t = task.go(s, () { () })
					fail.catch(() { task.join(t) })
					()
				},
			)
		})
		task.receive(gate)
		fail.raise("boom")
	})
}
