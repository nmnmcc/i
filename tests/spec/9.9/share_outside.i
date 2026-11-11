// spec: 9.9
// expect: stop "panic: task: go outside its scope"
// runtime.md 6.1: share checks membership exactly as go does, also in local scopes.
(
	i/task
)


main = () {
	task.scope((outer) {
		carrier: task.Channel(task.Scope(Never)) = task.channel()
		task.go(outer, () {
			scope = task.receive(carrier)
			task.share(scope, 7)
		})
		task.scope((inner) { task.send(carrier, inner) })
	})
}
