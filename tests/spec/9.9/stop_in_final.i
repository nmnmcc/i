// spec: 9.5, 9.9
// expect: stop "panic: final"
// A stop in one final does not run an outer final.
(
	i/task
	i/resource
)

main = () {
	task.scope((s) {
		child = task.go(s, () {
			resource.ensure(
				() {
					resource.ensure(
						() { Nil },
						() {
							task.yield()
							panic("final")
						},
					)
				},
				() { panic("outer must not run") },
			)
		})
		task.join(child)
	})
}
