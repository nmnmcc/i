// spec: 9.9
// expect: stop "panic: task: value is not portable"
// runtime.md 6.4: event construction checks a portable payload even if it loses.
(
	i/task
	i/control
)

main = () {
	control.boundary((b) {
		c = task.channel()
		task.choose([task.always(Nil), task.send_event(c, b)])
		Nil
	})
}
