// spec: 9.9
// expect: stop "panic: task: value is not portable"
// runtime.md D7: a portable send checks its payload at the call, before parking.
(
	i/task
	i/control
)


main = () {
	control.boundary((b) {
		c = task.channel()
		task.send(c, b)
	})
}
