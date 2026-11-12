// Design counterexample; not run. Give this function a live Stream.
// Force the first branch to enter a pending read before the second succeeds.
(
	i/net
	i/task
)
twice = (stream: net.Stream) {
	task.race(() { net.read(stream, 1) }, () { <<>> })
	net.read(stream, 1)
}
