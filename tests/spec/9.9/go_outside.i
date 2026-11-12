// spec: 9.9
// expect: stop "panic: task: go outside its scope"
// runtime.md R1: a sibling does not inherit a scope merely by receiving its
// handle. Membership is checked even if that scope has already ended.
(
	i/task
)

main = () {
	task.scope((outer) {
		carrier: task.Channel(task.Scope(Never)) = task.channel()
		task.go(outer, () {
			scope = task.receive(carrier)
			task.go(scope, () { Nil })
		})
		task.scope((inner) { task.send(carrier, inner) })
	})
}
