// spec: 9.9
// expect: stop "panic: task: value is not portable"
// runtime.md D7: child errors are checked before completion and arbitration.
(
	i/task
	i/control
	i/fail
)

@main
main = () {
	control.boundary((b) {
		fail.catch(() {
			task.scope((s) {
				task.go(s, () { fail.raise(b) })
				Nil
			})
		})
		Nil
	})
}
