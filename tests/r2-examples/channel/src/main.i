// runtime §6.1: channel is an unbuffered FIFO rendezvous.
(
	i/task
	(Io) = i/io
)

@main
main = (io ! Io) {
	task.scope((s) {
		c: task.Channel(String) = task.channel()
		task.go(s, () { task.send(c, "hello") })
		receiver = task.go(s, () { io.print(task.receive(c)) })
		task.join(receiver)
	})
}
