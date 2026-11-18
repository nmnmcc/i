// spec: 9.9
// expect: pass
// output: foreign-child-final
// Two nested driver hops must preserve the scope reply and its typed error.
// runtime.md R2/R3/6.3: retained callbacks keep their logical opener. Cancelling
// their scope drains nested scopes even when an inner sibling is running.
(
	i/task
	i/control
	i/resource
	i/fail
)

@host("test:io/v1")
Out = (print = (String) -> ())

print = (text : String) <- (out : Out) { out.print(text) }

// Decision 2: install the real key required by a kept pure callback.
without_fail = <A, N>(body : () <- (Fail(Never), ..N) -> A) -> A {
	body() <- (Fail(raise = (_ : Never) { panic("fixture: impossible error") }))
}

main = () {
	callback = control.keep((work : (() -> ()) -> ()) {
		task.scope((s) {
			gate : task.Channel(()) = task.channel()
			task.go(s, () {
				task.receive(gate)
				fail.raise("callback")
			})
			signal = control.keep((_ : ()) { task.send(gate, ()) })
			work(() { without_fail(() { signal(()) }) })
		})
	})
	assert(
		task.run(() {
			task.run(() {
				work = control.keep((notify : () -> ()) {
					task.scope((inner) {
						task.go(inner, () {
							resource.ensure(
								() {
									notify()
									task.sync(task.never())
								},
								() {
									task.yield()
									print("foreign-child-final")
								},
							)
						})
						task.sync(task.never())
					})
				})
				assert(
					fail.catch(() {
						callback((notify) { without_fail(() { work(notify) }) })
					}) ==
						Error("callback"),
				)
				42
			})
		}) ==
			42,
	)
}
