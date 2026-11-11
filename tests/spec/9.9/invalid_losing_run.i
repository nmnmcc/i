// spec: 9.9
// expect: stop "panic: task: handle of another run"
// runtime.md 6.4: every stateful leaf is prevalidated, even a losing alternative.
(
	i/task
)


main = () {
	c: task.Channel(Int) = task.channel()
	task.run(() {
		task.sync(task.choose([task.always(1), task.receive_event(c)]))
	})
}
