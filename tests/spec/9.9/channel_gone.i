// spec: 9.9
// expect: stop "panic: task: channel is gone"
// runtime.md R3/6.4: sync prevalidates even a losing leaf's channel liveness.
(
	i/task
)

@main
main = () {
	c: task.Channel(Int) = task.scope((_) { task.channel() })
	task.sync(task.choose([task.always(1), task.receive_event(c)]))
}
