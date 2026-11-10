// spec: 9.9
// expect: stop "panic: task: value is not portable"
// runtime.md D7: a portable child checks its result on completion, without join.
(
	i/task
	i/control
)

@main
main = () {
	control.boundary((b) {
		task.scope((s) {
			task.go(s, () { b })
			Nil
		})
	})
}
