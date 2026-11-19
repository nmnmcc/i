// spec: 9.5, 9.9
// expect: stop "panic: released on cancel"
(
	i/task
	i/resource
	i/fail
)

// A task blocked inside `scoped` is cancelled by its group: its release runs.
main = () {
	fail.catch(() {
		task.scope((s) {
			gate : task.Channel(()) = task.channel()
			task.go(s, () {
				resource.scoped(() {
					resource.acquire(
						() { 1 },
						(_) { panic("released on cancel") },
					)
					task.send(gate, ())
					task.sync(task.never())
				})
			})
			task.receive(gate)
			fail.raise("boom")
		})
	})
}
