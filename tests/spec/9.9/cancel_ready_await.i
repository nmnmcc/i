// spec: 9.9, 9.11
// expect: pass
// runtime.md 5.4/R4: cancellation at even an already-Ready await happens before
// take. Borrowed waits preserve that answer; owned waits discard it in the final.
(
	i/task
	i/request
	i/resource
	i/fail
)

probe = (wait: () <- (request.Await) -> Nil) {
	result = fail.catch(() {
		task.scope((s) {
			armed: task.Channel(Nil) = task.channel()
			task.go(s, () {
				task.send(armed, Nil)
				wait()
				panic("await continued")
			})
			task.go(s, () {
				task.receive(armed)
				fail.raise("boom")
			})
			task.sync(task.never())
		})
	})
	assert(result == Error("boom"))
}

main = () {
	borrowed = request.ready(Nil)
	probe(() { request.await(borrowed) })
	assert(request.state(borrowed) == request.Ready(Nil))
	owned = request.ready(Nil)
	probe(() { request.await_owned(owned) })
	assert(request.state(owned) == request.Cancelled)
}
