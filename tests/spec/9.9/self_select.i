// spec: 9.9
// expect: stop "panic: task: deadlock"
// runtime.md 6.4: alternatives in one registration never rendezvous together.
(
	i/task
)

@main
main = () {
	c: task.Channel(Nil) = task.channel()
	task.sync(task.choose([task.send_event(c, Nil), task.receive_event(c)]))
}
