// spec: 9.9
// expect: pass
// output: foreign-child-final
// runtime.md R2/R3/6.3: retained callbacks keep their logical opener. Cancelling
// their scope drains nested scopes even when an inner sibling is running.
(
	i/task
	i/control
	i/resource
	i/fail
)

@host("test:io/v1")
Out {
	Out(print: (text: String) -> Nil)
}

print = (text: String, out ! Out) { out.print(text) }


main = () {
	callback = control.keep_env((work: (() -> Nil) -> Nil) {
		task.scope((s) {
			gate: task.Channel(Nil) = task.channel()
			task.go(s, () {
				task.receive(gate)
				fail.raise("callback")
			})
			signal = control.keep_env((_: Nil) { task.send(gate, Nil) })
			work(() { signal(Nil) })
		})
	})
	assert(task.run(() {
		work = control.keep_env((notify: () -> Nil) {
			task.scope((inner) {
				task.go(inner, () {
					resource.ensure(() {
						notify()
						task.sync(task.never())
					}, () {
						task.yield()
						print("foreign-child-final")
					})
				})
				task.sync(task.never())
			})
		})
		assert(
			fail.catch(() { callback((notify) { work(notify) }) }) ==
				Error("callback")
		)
		42
	}) == 42)
}
