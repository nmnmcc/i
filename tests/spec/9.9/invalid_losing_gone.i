// spec: 9.9
// expect: stop "panic: task: channel is gone"
// runtime.md 6.4/R3: a dead channel is invalid in a losing alternative too.
(
	i/task
)

@main
main = () {
	c: task.Channel(Int) = task.scope((_) { task.channel() })
	task.sync(task.choose([task.always(1), task.receive_event(c)]))
}
