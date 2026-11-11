// spec: 9.9
// expect: stop "panic: task: value is not portable"
// runtime.md 6.1: sharing checks portability even in a local scope.
(
	i/task
	i/control
)


main = () {
	task.local_scope((s) {
		control.boundary((b) {
			task.share(s, b)
			Nil
		})
		Nil
	})
}
