// spec: 9.9, 9.11
// expect: pass
// output: echo cleanup
// runtime.md R4: finals retain services and can await, yield, open fresh scopes
// and rendezvous through the still-active task boundary before scope return.
(
	i/task
	i/request
	i/resource
	i/fail
)

@host("test:async/v1", "request")
Later {
	Later(echo: (text: String) -> Request(String))
}


main = (later ! Later) {
	result = fail.catch(() {
		task.scope((s) {
			armed: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(() {
					task.send(armed, Nil)
					task.sync(task.never())
				}, () {
					assert(
						request.await_owned(later.echo("cleanup")) == "cleanup"
					)
					task.yield()
					task.scope((fresh) {
						c: task.Channel(Nil) = task.channel()
						task.go(fresh, () { task.send(c, Nil) })
						task.receive(c)
					})
				})
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
