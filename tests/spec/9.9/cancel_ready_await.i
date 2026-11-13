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

probe = (wait: () -> Nil ! request.Await) {
	result = fail.catch(() {
		task.scope((s) {
			armed: task.Channel(Nil) = task.channel()
			task.go(s, () {
				task.send(armed, Nil)
				// §9.9: the failure may come before or after the await; the
				// child then waits until the cancellation reaches it.
				wait()
				task.sync(task.never())
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
	// §9.9: "Timing may choose ... which failures precede cancellation"; when
	// the await completed before the failure, it took the answer.
	after = request.state(borrowed)
	assert(after == request.Ready(Nil) || after == request.Taken)
	owned = request.ready(Nil)
	probe(() { request.await_owned(owned) })
	settled = request.state(owned)
	assert(settled == request.Cancelled || settled == request.Taken)
}
