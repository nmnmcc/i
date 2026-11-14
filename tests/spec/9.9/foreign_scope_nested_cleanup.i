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
Out = (print = (String) -> Nil)

print = (text : String) <- (out : Out) { out.print(text) }

// Decision 2: install the real key required by a kept pure callback.
without_fail = <A, N>(body : () <- (Fail(Never), ..N) -> A) -> A {
	body() <- (Fail(raise = (_ : Never) { panic("fixture: impossible error") }))
}

main = () {
	callback = control.keep((work : (() -> Nil) -> Nil) {
		task.scope((s) {
			gate : task.Channel(Nil) = task.channel()
			task.go(s, () {
				task.receive(gate)
				fail.raise("callback")
			})
			signal = control.keep((_ : Nil) { task.send(gate, Nil) })
			work(() { without_fail(() { signal(Nil) }) })
		})
	})
	assert(
		task.run(() {
			work = control.keep((notify : () -> Nil) {
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
		}) ==
			42,
	)
}
