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
	i/request
	i/fail
)

@host("test:io/v1")
Out {
	Out(print: (text: String) -> Nil)
}

print = (text: String, out ! Out) { out.print(text) }

probe = (label: String, wait: () -> Nil ! task.Scheduler | request.Root) {
	result = fail.catch(() {
		task.scope((s) {
			armed: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(() {
					task.send(armed, Nil)
					wait()
					panic("cancelled body continued")
				}, () {
					task.yield()
					print(label)
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

@main
main = () {
	probe("yield", () { task.yield() })
	c: task.Channel(Nil) = task.channel()
	probe("send", () { task.send(c, Nil) })
	probe("receive", () { task.receive(c) })
	probe("sync", () {
		task.sync(task.map(task.always(Nil), (_) {
			panic("map ran after cancellation")
		}))
	})
	result = fail.catch(() {
		task.scope((s) {
			target = task.go(s, () { task.sync(task.never()) })
			armed: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(() {
					task.send(armed, Nil)
					task.join(target)
				}, () {
					task.yield()
					print("join")
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
	probe("scope-end", () {
		task.scope((inner) {
			task.go(inner, () { task.sync(task.never()) })
			Nil
		})
	})
}
