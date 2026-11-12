// spec: 9.5, 9.9
// expect: stop "panic: body"
// Stops run no final functions, including under a task scheduler.
(
	i/task
	i/resource
)

main = () {
	task.scope((s) {
		child = task.go(s, () {
			resource.ensure(() { panic("body") }, () { panic("must not run") })
		})
		task.join(child)
	})
}
