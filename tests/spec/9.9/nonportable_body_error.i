// spec: 9.9
// expect: stop "panic: task: value is not portable"
// runtime.md D7: scope body errors are checked before failure arbitration.
(
	i/task
	i/control
	i/fail
)

@main
main = () {
	control.boundary((b) {
		fail.catch(() { task.scope((_) { fail.raise(b) }) })
		Nil
	})
}
