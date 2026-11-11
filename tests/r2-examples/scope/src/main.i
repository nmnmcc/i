// runtime §6.2: scope waits for all children; join returns the child's result.
(
	i/task
	i/core
	(Io) = i/io
)

@main
main = (io ! Io) {
	answer = task.scope((s) {
		a = task.go(s, () { 20 })
		b = task.go(s, () { 22 })
		task.join(a) + task.join(b)
	})
	io.print(core.inspect(answer))
}
