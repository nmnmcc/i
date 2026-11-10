// spec: 9.9
// expect: pass
// runtime.md 6.4: maps execute after commit, in the syncing task, may suspend
// or raise, and their error does not roll back the peer's committed operation.
(
	i/task
	i/fail
	i/request
)

@main
main = () {
	task.scope((s) {
		c: task.Channel(Int) = task.channel()
		ack: task.Channel(Nil) = task.channel()
		task.go(s, () {
			task.send(c, 7)
			task.send(ack, Nil)
		})
		event = task.map(task.receive_event(c), (x) {
			assert(x == 7)
			request.await_owned(request.ready(Nil))
			fail.raise("map")
		})
		assert(fail.catch(() { task.sync(event) }) == Error("map"))
		task.receive(ack)
	})
}
