// spec: 9.9
// expect: stop "panic: task: handle of another run"
// runtime.md R1: run identity is checked before every other handle condition.
(
	i/task
)


main = () {
	c: task.Channel(Int) = task.channel()
	task.run(() { task.send(c, 1) })
}
