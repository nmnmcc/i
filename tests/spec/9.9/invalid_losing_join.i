// spec: 9.9
// expect: stop "panic: task: join outside its scope"
// runtime.md 6.4/R1: all join chains are validated before registration, even
// when an earlier always leaf would otherwise be ready to win.
(
	i/task
)

@main
main = () {
	t = task.scope((s) { task.go(s, () { 7 }) })
	task.sync(task.choose([task.always(1), task.join_event(t)]))
}
