// runtime §6.4: empty program waitable set and no runnable task means deadlock.
(
	i/task
	(Io) = i/io
)

@main
main = (io ! Io) {
	task.scope((s) {
		c: task.Channel(String) = task.channel()
		receiver = task.go(s, () { io.print(task.receive(c)) })
		task.join(receiver)
	})
}
