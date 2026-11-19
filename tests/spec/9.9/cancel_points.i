// spec: 9.9
// expect: pass
// output: yield
// output: send
// output: receive
// output: sync
// output: join
// output: scope-end
// runtime.md R4: started tasks receive cancellation at their next suspension;
// finals may yield. R4a processes scope cancellation before cancelled joins.
(
	i/task
	i/request
	i/resource
	i/fail
)

@host("test:io/v1")
Out = (print = (String) -> ())

print = (text : String) <- (out : Out) { out.print(text) }

// §9.9 lets the failure come before or after any one step of the waiting
// child, so the child repeats its step until the cancellation reaches it.
yield_forever = () -> () {
	task.yield()
	yield_forever()
}

sync_forever = () -> () {
	task.sync(
		task.map(task.always(()), (_) { () }),
	)
	sync_forever()
}

probe = (
	label : String,
	wait : () <- (Fail(Never), task.Scheduler, request.Root) -> ()
) {
	result = fail.catch(() {
		task.scope((s) {
			armed : task.Channel(()) = task.channel()
			task.go(s, () {
				resource.ensure(
					() {
						task.send(armed, ())
						wait()
						panic("cancelled body continued")
					},
					() {
						task.yield()
						print(label)
					},
				)
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
	probe("yield", () { yield_forever() })
	c : task.Channel(()) = task.channel()
	probe("send", () { task.send(c, ()) })
	probe("receive", () { task.receive(c) })
	probe("sync", () { sync_forever() })
	result = fail.catch(() {
		task.scope((s) {
			target = task.go(s, () { task.sync(task.never()) })
			armed : task.Channel(()) = task.channel()
			task.go(s, () {
				resource.ensure(
					() {
						task.send(armed, ())
						task.join(target)
					},
					() {
						task.yield()
						print("join")
					},
				)
			})
			task.go(s, () {
				task.receive(armed)
				fail.raise("boom")
			})
			task.sync(task.never())
		})
	})
	assert(result == Error("boom"))
	probe("scope-end", () {
		task.scope((inner) {
			task.go(inner, () { task.sync(task.never()) })
			()
		})
	})
}
