// spec: 9.9
// expect: pass
// runtime.md 6.4: event construction is pure and does not check liveness;
// portability checks the ordinary stored descriptions, not live loop tables.
(
	i/task
	i/core
)


main = () {
	c: task.Channel(Int) = task.scope((_) { task.channel() })
	event = task.send_event(c, 7)
	assert(core.portable(event))
}
