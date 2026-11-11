// spec: 9.9
// expect: stop "panic: task: handle of another run"
// runtime.md R1/D7: direct send checks run identity before payload portability.
(
	i/task
	i/control
)

@main
main = () {
	control.boundary((b) {
		c = task.channel()
		task.run(() { task.send(c, b) })
	})
}
